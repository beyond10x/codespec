package codespec.read.groovy;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;

/**
 * The Groovy reader's tests, run as a plain program:
 *
 * <pre>java -cp &lt;classes&gt;:&lt;groovy-all jar&gt; codespec.read.groovy.ReaderTest &lt;repo root&gt; &lt;codespec binary&gt; &lt;work dir&gt;</pre>
 *
 * It prints one line per case and a summary {@code N passed, M failed, K skipped}, and exits 1 when any case
 * fails.
 *
 * <p>Each fixture under {@code testdata/refuse/} marks one line {@code // refused <category>}. The
 * reader must refuse at that line with a reason beginning {@code <category>: }; where the line is a
 * statement, it must also be a {@code foreign} statement there. A declaration (a method, an import,
 * a property) has no foreign form, so for those the refusal alone is checked, and the test checks
 * that the declaration was not read.
 */
public final class ReaderTest {
    /** The closed list of refusal categories, in the order the README lists them. */
    static final List<String> CATEGORIES = List.of(
        "dynamic", "closure", "groovy-semantics", "type", "unresolved", "gorm", "spring", "unsupported");

    private static final String INT32 = "{\"k\":\"int\",\"width\":\"32\",\"signed\":true}";
    private static final String INT64 = "{\"k\":\"int\",\"width\":\"64\",\"signed\":true}";

    private final Path root;
    private final Path codespec;
    private final Path work;
    private final Path testdata;

    private ReaderTest(Path root, Path codespec, Path work) {
        this.root = root;
        this.codespec = codespec;
        this.work = work;
        this.testdata = root.resolve("adapters/groovy/testdata");
    }

    interface Case {
        void run() throws Exception;
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 3 && args.length != 4) {
            System.err.println("usage: ReaderTest <repo root> <codespec binary> <work dir> [<case name substring>]");
            System.exit(2);
        }
        ReaderTest t = new ReaderTest(Path.of(args[0]), Path.of(args[1]), Path.of(args[2]));
        Files.createDirectories(t.work);
        Map<String, Case> cases = new LinkedHashMap<>();
        cases.put("typed_reads_with_no_refusal", t::typedReadsWithNoRefusal);
        cases.put("typed_read_twice_gives_the_same_bytes", t::typedReadTwiceGivesTheSameBytes);
        cases.put("typed_document_is_canonical_cir_1", t::typedDocumentIsCanonical);
        cases.put("typed_holds_every_construct_of_the_story", t::typedHoldsEveryConstruct);
        cases.put("after_the_passes_the_hash_map_loop_is_unordered_and_the_tree_map_loop_is_not", t::hashMapLoopIsUnordered);
        cases.put("integer_max_value_plus_one_is_int32_and_a_long_field_int64", t::integerMaxValuePlusOneIsInt32);
        cases.put("a_def_local_is_typed_from_its_inferred_type", t::aDefLocalIsTypedFromItsInferredType);
        cases.put("the_compile_static_class_and_the_unannotated_class_are_both_read", t::bothClassesAreRead);
        cases.put("the_reader_version_is_the_workspace_version", t::theReaderVersionIsTheWorkspaceVersion);
        cases.put("the_manifest_claims_only_features_the_typed_read_uses", t::theManifestClaimsOnlyUsedFeatures);
        cases.put("a_def_parameter_called_dynamically_is_foreign_dynamic", () -> t.foreignAt("def-param"));
        cases.put("a_method_missing_declared_is_refused_dynamic", () -> t.declarationRefused("method-missing", "Desk.methodMissing"));
        cases.put("a_meta_class_assignment_is_foreign_dynamic", () -> t.foreignAt("meta-class"));
        cases.put("a_closure_passed_to_each_is_foreign_closure", () -> t.foreignAt("closure-each"));
        cases.put("groovy_truth_in_an_if_is_foreign_groovy_semantics", () -> t.foreignAt("groovy-truth"));
        cases.put("safe_navigation_is_foreign_groovy_semantics", () -> t.foreignAt("safe-navigation"));
        cases.put("the_elvis_operator_is_foreign_groovy_semantics", () -> t.foreignAt("elvis"));
        cases.put("int_divided_by_int_is_foreign_groovy_semantics", () -> t.foreignAt("int-division"));
        cases.put("intdiv_of_two_longs_or_two_ints_reads_as_op_div_of_their_type", t::intdivReadsAsOpDiv);
        cases.put("x_assigned_x_intdiv_y_reads_as_assign_div_and_y_assigned_x_intdiv_z_as_an_assignment_of_op_div",
            t::intdivAssignmentReadsAsAssignDiv);
        cases.put("intdiv_of_an_int_and_a_long_is_foreign_groovy_semantics", () -> t.foreignAt("intdiv-mixed"));
        cases.put("intdiv_of_an_int_literal_over_a_long_reads_as_int64_op_div", t::intdivOfALiteralReceiverIsInt64);
        cases.put("intdiv_of_two_shorts_is_foreign_groovy_semantics", () -> t.foreignAt("intdiv-short"));
        cases.put("a_field_assigned_its_own_intdiv_through_no_call_reads_as_assign_div_and_through_a_call_as_an_assignment",
            t::intdivFieldTargets);
        cases.put("groovy_intdiv_truncates_toward_zero_wraps_and_throws_on_zero_as_go_divides",
            ReaderTest::groovyIntdivDividesAsGo);
        cases.put("adversary_intdiv_as_an_expression_statement_is_foreign_unsupported_as_a_plus_b_is",
            () -> t.foreignAt("intdiv-statement"));
        cases.put("adversary_intdiv_assigned_to_a_long_in_a_compile_static_class_is_foreign_as_groovyc_refuses_it",
            () -> t.foreignAt("intdiv-compile-static"));
        cases.put("a_default_parameter_value_is_refused_groovy_semantics", () -> t.declarationRefused("default-param", "Desk.scaled"));
        cases.put("a_big_decimal_literal_is_foreign_type_with_the_shared_text", t::aBigDecimalLiteralIsForeignType);
        cases.put("an_unresolved_import_refuses_its_file_and_the_rest_is_read", t::anUnresolvedImportRefusesItsFile);
        cases.put("a_string_property_without_an_initializer_is_refused_type", t::aStringPropertyIsRefusedType);
        cases.put("every_refusal_of_every_fixture_begins_with_a_category", t::everyRefusalBeginsWithACategory);
        cases.put("the_readme_lists_the_eight_categories_each_with_an_example", t::theReadmeListsTheCategories);
        // Adversary: Groovy code the reader must not read into cir/1 with a meaning Groovy does not
        // give it. Each fixture under testdata/adversary marks with "// diverges" the lines whose
        // read differs from a Groovy 2.4.10 run; each case passes when the reader refuses one of
        // them, or reads none of them in the way named.
        cases.put("string_equals_of_a_gstring_is_not_read_as_string_equality",
            () -> t.notReadAs("string-equals-gstring", s -> Json.write(s).contains("\"op\": \"==\"")));
        cases.put("a_property_write_through_a_declared_setter_is_not_read_as_a_field_write",
            () -> t.notReadAs("setter-dynamic", ReaderTest::writesFieldCopies));
        cases.put("a_compile_static_property_write_through_a_declared_setter_is_not_read_as_a_field_write",
            () -> t.notReadAs("setter-compile-static", ReaderTest::writesFieldCopies));
        cases.put("a_write_to_a_final_property_which_groovy_refuses_at_run_time_is_not_read_as_a_field_write",
            () -> t.notReadAs("final-property", ReaderTest::writesFieldCopies));
        cases.put("a_property_of_a_class_declaring_get_property_or_set_property_is_not_read_as_its_field",
            () -> t.notReadAs("property-hooks", s -> Json.write(s).contains("\"name\": \"copies\"")));
        cases.put("a_def_local_is_not_read_as_long_before_groovy_widens_it",
            () -> t.notReadAs("def-widened", s -> Json.write(s).contains("\"width\": \"64\"")));
        cases.put("a_list_grown_through_a_method_while_a_for_each_iterates_it_is_not_read_as_a_loop_that_runs_on",
            () -> t.notReadAs("modified-while-iterated", s -> true));
        // Grails: the lending application under testdata/lending.
        cases.put("lending_read_matches_expected_md_and_the_markers_per_artefact", t::lendingMatchesExpected);
        cases.put("lending_read_twice_gives_the_same_canonical_bytes", t::lendingReadTwiceIsCanonical);
        cases.put("lending_read_equals_the_checked_in_document", t::lendingEqualsTheCheckedInDocument);
        cases.put("lending_resolve_only_files_yield_nothing_and_due_dates_resolves", t::lendingResolveOnlyYieldsNothing);
        cases.put("lending_a_tool_string_property_is_a_pointer_with_zero_value_nil", t::lendingToolNameIsAPointer);
        cases.put("lending_find_by_name_and_save_are_foreign_gorm_at_their_lines", t::lendingGormCallsAreForeign);
        cases.put("lending_spring_injected_fields_and_calls_through_them_are_refused_spring", t::lendingSpringInjection);
        cases.put("lending_domain_metadata_is_refused_gorm_mapping_metadata", t::lendingDomainMetadata);
        cases.put("lending_without_the_grails_jars_the_transactional_service_is_refused_unresolved_at_its_import",
            t::lendingTransactionalUnresolvedWithoutJars);
        cases.put("lending_every_reason_begins_with_a_category", t::lendingEveryReasonHasACategory);
        cases.put("a_directory_without_application_properties_reads_as_plain_groovy", t::plainDirectoryReadsAsGroovy);
        cases.put("two_artefacts_with_one_simple_name_are_both_refused", t::duplicateSimpleNamesAreRefused);
        cases.put("the_readme_names_the_grails_jars_as_grails_home_paths", t::theReadmeNamesTheGrailsJars);
        cases.put("lending_with_the_grails_jars_the_transactional_service_reads", t::lendingTransactionalWithJars);
        cases.put("lending_with_the_grails_jars_no_identifier_of_the_transactional_transform_is_in_the_read",
            t::lendingNoTransformIdentifier);
        // Adversary (story:grails-artefacts): Grails 2.5.6 code read wrong, or refused for a reason
        // that is not why it cannot be read. Each case builds a small application under the work dir.
        cases.put("adversary_grails_a_service_injected_grails_application_is_spring_and_its_other_methods_are_read",
            t::adversaryGrailsApplicationInjected);
        cases.put("adversary_grails_a_domain_string_property_written_or_compared_with_null_is_read_as_the_java_reader_reads_it",
            t::adversaryNullablePropertyUses);
        cases.put("adversary_grails_a_service_naming_a_src_java_class_is_not_refused_unresolved",
            t::adversarySrcJavaResolves);
        cases.put("adversary_grails_with_the_grails_jars_an_entity_class_in_src_groovy_does_not_fail_the_read",
            t::adversaryEntityInSrcGroovy);
        cases.put("adversary_grails_an_initialised_service_field_that_spring_replaces_is_not_read_as_the_services_own_instance",
            t::adversaryInitialisedServiceField);
        cases.put("adversary_grails_with_the_grails_jars_an_overload_the_transform_did_not_rewrite_is_not_refused_as_rewritten",
            t::adversaryOverloadNotRewritten);
        cases.put("adversary_grails_list_order_by_on_a_domain_class_is_foreign_gorm", t::adversaryListOrderBy);
        cases.put("a_method_whose_quoted_name_is_no_identifier_is_refused_and_the_document_reads_strictly",
            t::aQuotedMethodNameIsRefused);
        // A class, a jar or a file that does not resolve refuses a file and the read writes its
        // document. Each case builds its application, and the jars it needs, under the work dir.
        cases.put("a_src_java_class_extending_a_class_of_a_classpath_jar_is_defined_and_the_service_naming_it_reads",
            t::aSrcJavaClassExtendingAClasspathClassResolves);
        cases.put("a_class_whose_own_dependency_is_on_no_jar_refuses_the_service_importing_it_unresolved",
            t::aClassWhoseDependencyIsMissingRefusesTheFileNamingIt);
        cases.put("a_class_reached_through_a_call_whose_own_dependency_is_on_no_jar_refuses_the_service_reaching_it_unresolved",
            t::aClassReachedThroughACallWhoseDependencyIsMissingRefusesTheFileReachingIt);
        cases.put("a_transformation_that_needs_a_class_on_no_jar_refuses_the_class_it_annotates_unresolved",
            t::aTransformationThatNeedsAMissingClassRefusesTheClassItAnnotates);
        cases.put("a_stub_class_without_a_code_attribute_refuses_the_service_importing_it_unresolved",
            t::aStubClassWithoutCodeRefusesTheFileNamingIt);
        cases.put("an_annotation_constant_of_a_class_on_no_jar_refuses_its_file_unresolved",
            t::anAnnotationConstantOfAMissingClassRefusesItsFile);
        cases.put("a_qualified_reference_in_a_static_method_to_an_enum_left_out_refuses_its_file_unresolved",
            t::aQualifiedReferenceToALeftOutEnumRefusesItsFile);
        cases.put("a_reference_in_a_static_method_to_a_refused_domain_class_of_its_package_refuses_its_file_unresolved",
            t::aReferenceToARefusedDomainClassOfItsPackageRefusesItsFile);
        // The general rule: whatever fails while one file is compiled or read refuses that file
        // with the reason, a resolve-only file is left out, every other artefact is read and the
        // document is written. No case here is a message the reader knows.
        cases.put("a_logging_annotation_whose_logger_class_is_on_no_jar_refuses_its_file_and_a_resolve_only_file_is_left_out",
            t::aLoggingAnnotationWithoutItsLoggerRefusesItsFile);
        cases.put("a_trait_super_call_whose_trait_s_file_was_left_out_refuses_its_file_with_the_compiler_s_message",
            t::aTraitSuperCallWhoseTraitWasLeftOutRefusesItsFile);
        cases.put("a_transformation_of_a_jar_that_throws_as_it_runs_is_not_run_and_the_classes_it_annotates_are_read",
            t::aTransformationOfAJarIsNotRun);
        cases.put("a_compile_error_the_reader_has_never_seen_refuses_its_file_with_the_compiler_s_message",
            t::aCompileErrorNeverSeenRefusesItsFile);
        cases.put("an_exception_thrown_inside_a_compilation_phase_refuses_the_file_being_compiled_with_its_class_and_message",
            t::anExceptionInsideAPhaseRefusesTheFileBeingCompiled);
        cases.put("an_exception_that_names_no_file_refuses_every_artefact_with_one_reason_and_the_document_is_written",
            t::anExceptionThatNamesNoFileRefusesEveryArtefact);
        cases.put("a_chain_of_three_files_each_failing_once_the_next_is_gone_ends_with_each_refused_once",
            t::aChainOfThreeFilesEndsWithEachRefusedOnce);
        cases.put("a_class_of_a_jar_that_needs_javax_activation_or_jaxb_which_this_jdk_does_not_ship_refuses_the_service_naming_it",
            t::aClassThatNeedsARemovedJdkPackageRefusesTheFileNamingIt);
        cases.put("a_service_that_is_not_utf_8_text_is_refused_and_the_rest_is_read", t::aServiceThatIsNotUtf8IsRefused);
        // Call lists: the holds application under testdata/holds. Code the reader refuses stays in
        // the call graph: a foreign statement lists its calls, a refused method or a method of a
        // refused class is an opaque function.
        cases.put("holds_a_foreign_statement_lists_its_calls_a_call_before_its_receiver_and_arguments_a_closure_as_a_lambda",
            t::holdsForeignStatementsListTheirCalls);
        cases.put("holds_a_method_refused_at_its_declaration_is_an_opaque_function_with_its_reason_and_calls",
            t::holdsARefusedMethodIsOpaque);
        cases.put("holds_every_method_and_constructor_of_a_class_refused_as_a_whole_is_an_opaque_function",
            t::holdsARefusedClassIsOpaque);
        cases.put("holds_two_overloads_are_two_functions_and_a_call_names_the_one_the_type_checker_picked",
            t::holdsOverloadsAreTwoFunctions);
        cases.put("holds_reach_from_check_in_reaches_the_notice_method_called_only_from_a_refused_method",
            t::holdsReachFromCheckIn);
        cases.put("holds_read_twice_gives_the_same_canonical_bytes_and_codespec_check_accepts_them",
            t::holdsReadTwiceIsCanonical);
        cases.put("holds_read_equals_the_checked_in_document", t::holdsEqualsTheCheckedInDocument);
        cases.put("every_foreign_statement_of_every_fixture_carries_a_call_list", t::everyForeignStatementCarriesACallList);
        cases.put("a_call_ref_names_what_the_type_checker_resolved_in_and_outside_the_module", t::callRefsNameTheirTargets);
        cases.put("a_class_refused_at_its_fields_and_a_method_named_by_no_identifier_are_opaque_and_the_document_reads_strictly",
            t::fieldRefusedClassAndQuotedNameAreOpaque);
        cases.put("classes_with_one_simple_name_yield_no_opaque_function_and_a_call_of_one_is_a_value_call",
            t::duplicateSimpleNamesYieldNoOpaqueFunction);
        cases.put("holds_a_gorm_method_on_a_domain_class_is_a_call_of_grails_gorm_marked_by_its_convention",
            t::holdsGormCallsAreMarked);
        cases.put("holds_a_call_through_an_x_service_field_is_a_call_of_the_service_s_method_marked_by_its_convention",
            t::holdsServiceFieldCallsAreMarked);
        cases.put("holds_reach_follows_a_convention_edge_and_a_query_selects_or_excludes_them", t::holdsReachAndQueryOverConventions);
        cases.put("a_convention_names_no_target_it_cannot_tell_and_none_outside_a_grails_application", t::conventionsDoNotGuess);
        cases.put("the_gorm_method_list_holds_every_method_of_the_gorm_api_classes_of_the_grails_distribution",
            t::gormListIsTheDistributions);
        cases.put("the_readme_names_both_conventions_their_marks_and_every_gorm_method", t::readmeNamesTheConventions);
        String only = args.length == 4 ? args[3] : null;
        if (only != null) {
            cases.keySet().removeIf(k -> !k.contains(only));
        }
        int passed = 0;
        int failed = 0;
        int skipped = 0;
        for (Map.Entry<String, Case> c : cases.entrySet()) {
            try {
                c.getValue().run();
                System.out.println("ok   " + c.getKey());
                passed++;
            } catch (Skip s) {
                System.out.println("skip " + c.getKey() + ": " + s.getMessage());
                skipped++;
            } catch (Throwable e) {
                System.out.println("FAIL " + c.getKey() + ": " + e);
                failed++;
            }
        }
        System.out.println(passed + " passed, " + failed + " failed, " + skipped + " skipped");
        System.exit(failed == 0 ? 0 : 1);
    }

    // ---- typed ----

    private Map<String, Object> typed() throws Exception {
        return Reader.read(testdata.resolve("typed"));
    }

    private void typedReadsWithNoRefusal() throws Exception {
        Map<String, Object> doc = typed();
        check(!doc.containsKey("refusals"), "refusals: " + Json.write(doc.getOrDefault("refusals", List.of())));
        check("cir/1".equals(doc.get("format")), "format");
        check("typed".equals(doc.get("name")), "name " + doc.get("name"));
        Map<String, Object> reader = map(doc.get("reader"));
        check("groovy".equals(reader.get("language")), "reader.language");
        check("codespec-read-groovy".equals(reader.get("name")), "reader.name");
        check(foreignPositions(doc.get("funcs")).isEmpty(), "foreign statements at " + foreignPositions(doc.get("funcs")));
    }

    private void typedReadTwiceGivesTheSameBytes() throws Exception {
        String a = Json.write(typed());
        String b = Json.write(typed());
        check(a.equals(b), "two reads differ");
    }

    /** The adapter's own bytes, before the CLI re-encodes them, are canonical cir/1. */
    private void typedDocumentIsCanonical() throws Exception {
        Path doc = work.resolve("typed.raw.json");
        Files.writeString(doc, Json.write(typed()), StandardCharsets.UTF_8);
        Process p = new ProcessBuilder(codespec.toString(), "validate", "--canonical", doc.toString()).inheritIO().start();
        check(p.waitFor() == 0, "codespec validate --canonical failed");
    }

    private void typedHoldsEveryConstruct() throws Exception {
        Map<String, Object> doc = typed();
        Map<String, Object> kind = byName(list(doc.get("types")), "Kind");
        check("enum".equals(kind.get("k")) && list(kind.get("variants")).size() == 2, "Kind " + Json.write(kind));
        Map<String, Object> tally = byName(list(doc.get("types")), "Tally");
        check("struct".equals(tally.get("k")), "Tally is " + tally.get("k"));
        String funcs = Json.write(doc.get("funcs"));
        for (String needle : new String[] {
            "\"k\": \"switch\"", "\"k\": \"for\"", "\"k\": \"break\"", "\"k\": \"for_range\"", "\"name\": \"sprintf\"",
            "\"name\": \"append\"", "\"name\": \"sorted_keys\"", "\"k\": \"incr\"",
        }) {
            check(funcs.contains(needle), "no " + needle + " in funcs");
        }
        Object describe = returned(funcByKey(doc, "describe"));
        check(same(describe, "{\"k\":\"intrinsic\",\"name\":\"sprintf\",\"args\":["
            + "{\"k\":\"lit\",\"value\":\"\\\"%s has %d\\\"\",\"t\":{\"k\":\"string\"}},"
            + "{\"k\":\"var\",\"name\":\"name\",\"t\":{\"k\":\"string\"}},"
            + "{\"k\":\"var\",\"name\":\"count\",\"t\":" + INT32 + "}],\"t\":{\"k\":\"string\"}}"),
            "describe returns " + Json.write(describe));
    }

    private void hashMapLoopIsUnordered() throws Exception {
        Path read = work.resolve("typed.read.json");
        Path passed = work.resolve("typed.pass.json");
        Files.writeString(read, Json.write(typed()), StandardCharsets.UTF_8);
        Process p = new ProcessBuilder(codespec.toString(), "pass", read.toString(), "--out", passed.toString()).inheritIO().start();
        check(p.waitFor() == 0, "codespec pass failed");
        Map<String, Object> facts = map(map(Json.parse(Files.readString(passed))).get("facts"));
        List<Object> unordered = list(facts.get("unordered_iteration"));
        Path tally = testdata.resolve("typed/Tally.groovy");
        String hashLoop = "Tally.groovy:" + lineOf(tally, "for (String k : counts.keySet())");
        String treeLoop = "Tally.groovy:" + lineOf(tally, "for (String k : sorted.keySet())");
        check(unordered.contains(hashLoop), "unordered_iteration " + unordered + " lacks the HashMap loop " + hashLoop);
        check(!unordered.contains(treeLoop), "unordered_iteration " + unordered + " holds the TreeMap loop " + treeLoop);
    }

    private void integerMaxValuePlusOneIsInt32() throws Exception {
        Map<String, Object> doc = typed();
        Object wrap = returned(funcByKey(doc, "wrap"));
        check(same(wrap, "{\"k\":\"binop\",\"op\":\"+\",\"x\":{\"k\":\"lit\",\"value\":\"2147483647\",\"t\":" + INT32
            + "},\"y\":{\"k\":\"lit\",\"value\":\"1\",\"t\":" + INT32 + "},\"t\":" + INT32 + "}"), "wrap returns " + Json.write(wrap));
        Map<String, Object> tally = byName(list(doc.get("types")), "Tally");
        check(same(byName(list(tally.get("fields")), "total").get("t"), INT64), "Tally.total is not int64");
        check(same(byName(list(tally.get("fields")), "counter").get("t"), INT32), "Tally.counter is not int32");
    }

    private void aDefLocalIsTypedFromItsInferredType() throws Exception {
        List<Object> body = list(funcByKey(typed(), "doubled").get("body"));
        Map<String, Object> let = map(body.get(0));
        check("let".equals(let.get("k")) && List.of("twice").equals(let.get("names")), "first statement " + Json.write(let));
        check(same(map(list(let.get("rhs")).get(0)).get("t"), INT32), "twice is not int32: " + Json.write(let));
    }

    private void bothClassesAreRead() throws Exception {
        Map<String, Object> doc = typed();
        check(funcByKeyOrNull(doc, "firstOver") != null, "Label.firstOver (@CompileStatic) was not read");
        check(funcByKeyOrNull(doc, "Tally.countOf") != null, "Tally.countOf (unannotated) was not read");
    }

    private void theReaderVersionIsTheWorkspaceVersion() throws Exception {
        String version = null;
        boolean inPackage = false;
        for (String line : Files.readAllLines(root.resolve("Cargo.toml"))) {
            if (line.startsWith("[")) {
                inPackage = line.equals("[workspace.package]");
            } else if (inPackage && line.startsWith("version = ")) {
                version = line.substring("version = ".length()).replace("\"", "").trim();
            }
        }
        check(Main.VERSION.equals(version), "Main.VERSION " + Main.VERSION + ", Cargo.toml " + version);
        check(Main.VERSION.equals(map(typed().get("reader")).get("version")), "reader.version");
    }

    /** The manifest is the reader's own JSON; its claim is checked against the read by codespec-conformance. */
    private void theManifestClaimsOnlyUsedFeatures() throws Exception {
        Map<String, Object> m = map(Json.parse(Main.describe()));
        check(Main.describe().equals(Json.write(m)), "the manifest is not in canonical layout");
        check("codespec-adapter/1".equals(m.get("format")) && "reader".equals(m.get("role")) && "groovy".equals(m.get("lang")),
            "manifest head " + Json.write(m));
        for (Object f : list(m.get("features"))) {
            check(!((String) f).startsWith("fact."), "a reader claims " + f);
        }
    }

    // ---- refusals ----

    /** {@code <file>:<line>} and category of the one line marked {@code // refused <category>} in a fixture. */
    private String[] marker(String fixture) throws IOException {
        List<String[]> found = new ArrayList<>();
        try (Stream<Path> files = Files.list(testdata.resolve("refuse/" + fixture))) {
            for (Path f : files.sorted().toList()) {
                List<String> lines = Files.readAllLines(f);
                for (int i = 0; i < lines.size(); i++) {
                    int at = lines.get(i).indexOf("// refused ");
                    if (at >= 0) {
                        found.add(new String[] {f.getFileName() + ":" + (i + 1), lines.get(i).substring(at + "// refused ".length()).trim()});
                    }
                }
            }
        }
        check(found.size() == 1, "markers in " + fixture + ": " + found.size());
        return found.get(0);
    }

    private Map<String, Object> refusalAtMarker(String fixture) throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("refuse/" + fixture));
        String[] m = marker(fixture);
        List<String> at = new ArrayList<>();
        for (Object r : list(doc.get("refusals"))) {
            Map<String, Object> rm = map(r);
            at.add(rm.get("pos") + " " + rm.get("reason"));
            if (m[0].equals(rm.get("pos")) && ((String) rm.get("reason")).startsWith(m[1] + ": ")) {
                return doc;
            }
        }
        throw new AssertionError("no refusal " + m[1] + " at " + m[0] + "; refusals " + at);
    }

    private void foreignAt(String fixture) throws Exception {
        Map<String, Object> doc = refusalAtMarker(fixture);
        String pos = marker(fixture)[0];
        check(foreignPositions(doc.get("funcs")).contains(pos), "no foreign statement at " + pos);
    }

    private void declarationRefused(String fixture, String key) throws Exception {
        Map<String, Object> doc = refusalAtMarker(fixture);
        check(funcByKeyOrNull(doc, key) == null, key + " was read");
        check(funcByKeyOrNull(doc, "Desk.one") != null, "the method beside it was not read");
    }

    private void aBigDecimalLiteralIsForeignType() throws Exception {
        foreignAt("big-decimal");
        Map<String, Object> doc = Reader.read(testdata.resolve("refuse/big-decimal"));
        String reason = (String) map(list(doc.get("refusals")).get(0)).get("reason");
        check(reason.equals("type: type java.math.BigDecimal is outside the reader's types"), "reason " + reason);
    }

    // ---- intdiv ----

    private static final String INTDIV = "package intdiv\n\nfinal class Case {\n"
        + "    static long div(long a, long b) {\n        return a.intdiv(b)\n    }\n\n"
        + "    static int div32(int a, int b) {\n        return a.intdiv(b)\n    }\n\n"
        + "    static long divBy(long a, long b) {\n        a = a.intdiv(b)\n        return a\n    }\n\n"
        + "    static long quotient(long a, long b) {\n        long q = 0\n        q = a.intdiv(b)\n        return q\n    }\n\n"
        + "    static long seven(long b) {\n        return 7.intdiv(b)\n    }\n}\n\n"
        + "final class Till {\n    long total = 0\n\n"
        + "    void halve(long b) {\n        total = total.intdiv(b)\n    }\n\n"
        + "    void halveOther(Till o, long b) {\n        o.total = o.total.intdiv(b)\n    }\n\n"
        + "    Till self() {\n        return this\n    }\n\n"
        + "    void halveThroughCall(long b) {\n        self().total = self().total.intdiv(b)\n    }\n}\n";

    private Map<String, Object> intdiv() throws Exception {
        Path dir = work.resolve("intdiv");
        write(dir.resolve("Case.groovy"), INTDIV);
        Map<String, Object> doc = Reader.read(dir);
        check(!doc.containsKey("refusals"), "refusals: " + Json.write(doc.getOrDefault("refusals", List.of())));
        return doc;
    }

    private static String var(String name, String t) {
        return "{\"k\":\"var\",\"name\":\"" + name + "\",\"t\":" + t + "}";
    }

    private void intdivReadsAsOpDiv() throws Exception {
        Map<String, Object> doc = intdiv();
        Object div = returned(funcByKey(doc, "div"));
        check(same(div, "{\"k\":\"binop\",\"op\":\"/\",\"x\":" + var("a", INT64) + ",\"y\":" + var("b", INT64) + ",\"t\":" + INT64 + "}"),
            "div returns " + Json.write(div));
        Object div32 = returned(funcByKey(doc, "div32"));
        check(same(div32, "{\"k\":\"binop\",\"op\":\"/\",\"x\":" + var("a", INT32) + ",\"y\":" + var("b", INT32) + ",\"t\":" + INT32 + "}"),
            "div32 returns " + Json.write(div32));
    }

    private void intdivAssignmentReadsAsAssignDiv() throws Exception {
        Map<String, Object> doc = intdiv();
        Map<String, Object> divBy = map(list(funcByKey(doc, "divBy").get("body")).get(0));
        check("assign".equals(divBy.get("k")) && "/=".equals(divBy.get("op")), "divBy's first statement " + Json.write(divBy));
        check(same(divBy.get("lhs"), "[" + var("a", INT64) + "]") && same(divBy.get("rhs"), "[" + var("b", INT64) + "]"),
            "divBy's first statement " + Json.write(divBy));
        Map<String, Object> q = map(list(funcByKey(doc, "quotient").get("body")).get(1));
        check("assign".equals(q.get("k")) && !q.containsKey("op"), "quotient's second statement " + Json.write(q));
        check(same(q.get("rhs"), "[{\"k\":\"binop\",\"op\":\"/\",\"x\":" + var("a", INT64) + ",\"y\":" + var("b", INT64)
            + ",\"t\":" + INT64 + "}]"), "quotient's second statement " + Json.write(q));
    }

    /** {@code 7.intdiv(b)}, an int literal over a long: Groovy divides in long, so the literal is int64. */
    private void intdivOfALiteralReceiverIsInt64() throws Exception {
        Object seven = returned(funcByKey(intdiv(), "seven"));
        check(same(seven, "{\"k\":\"binop\",\"op\":\"/\",\"x\":{\"k\":\"lit\",\"value\":\"7\",\"t\":" + INT64 + "},\"y\":"
            + var("b", INT64) + ",\"t\":" + INT64 + "}"), "seven returns " + Json.write(seven));
    }

    /**
     * A field target read through no call, {@code this} or a local, is {@code /=}; one read through a
     * call, evaluated twice by Groovy, is an assignment of the division.
     */
    private void intdivFieldTargets() throws Exception {
        Map<String, Object> doc = intdiv();
        for (String fn : List.of("Till.halve", "Till.halveOther")) {
            Map<String, Object> st = map(list(funcByKey(doc, fn).get("body")).get(0));
            check("assign".equals(st.get("k")) && "/=".equals(st.get("op")), fn + "'s statement " + Json.write(st));
            check("field".equals(map(list(st.get("lhs")).get(0)).get("k")), fn + "'s target " + Json.write(st));
            check(same(st.get("rhs"), "[" + var("b", INT64) + "]"), fn + "'s statement " + Json.write(st));
        }
        Map<String, Object> st = map(list(funcByKey(doc, "Till.halveThroughCall").get("body")).get(0));
        check("assign".equals(st.get("k")) && !st.containsKey("op"), "halveThroughCall's statement " + Json.write(st));
        check("binop".equals(map(list(st.get("rhs")).get(0)).get("k")), "halveThroughCall's statement " + Json.write(st));
    }

    /**
     * What Groovy 2.4.10 itself gives for {@code intdiv}, run through {@code GroovyShell}: Go's
     * integer division of one type, truncating toward zero, the minimum divided by -1 wrapping to
     * itself, and division by zero throwing where Go panics.
     */
    private static void groovyIntdivDividesAsGo() {
        groovy.lang.GroovyShell sh = new groovy.lang.GroovyShell();
        check(Long.valueOf(-3).equals(sh.evaluate("(-7L).intdiv(2L)")), "(-7L).intdiv(2L)");
        check(Long.valueOf(3).equals(sh.evaluate("7L.intdiv(2L)")), "7L.intdiv(2L)");
        check(Integer.valueOf(-3).equals(sh.evaluate("(-7).intdiv(2)")), "(-7).intdiv(2) is not the int -3");
        check(Long.valueOf(Long.MIN_VALUE).equals(sh.evaluate("Long.MIN_VALUE.intdiv(-1L)")), "Long.MIN_VALUE.intdiv(-1L)");
        check(Integer.valueOf(Integer.MIN_VALUE).equals(sh.evaluate("Integer.MIN_VALUE.intdiv(-1)")), "Integer.MIN_VALUE.intdiv(-1)");
        for (String zero : List.of("7L.intdiv(0L)", "7.intdiv(0)")) {
            try {
                sh.evaluate(zero);
                check(false, zero + " did not throw");
            } catch (ArithmeticException e) {
                // Go panics: integer divide by zero
            }
        }
    }

    private void anUnresolvedImportRefusesItsFile() throws Exception {
        Map<String, Object> doc = refusalAtMarker("unresolved-import");
        check(list(doc.get("refusals")).size() == 1, "refusals " + Json.write(doc.get("refusals")));
        String reason = (String) map(list(doc.get("refusals")).get(0)).get("reason");
        check(reason.equals("unresolved: com.example.missing.Gone"), "reason " + reason);
        check(byNameOrNull(list(doc.get("types")), "Desk") == null, "Desk, whose import does not resolve, was read");
        check(funcByKeyOrNull(doc, "Shelf.two") != null, "Shelf, beside it, was not read");
    }

    private void aStringPropertyIsRefusedType() throws Exception {
        Map<String, Object> doc = refusalAtMarker("string-property");
        String reason = (String) map(list(doc.get("refusals")).get(0)).get("reason");
        check(reason.contains("title") && reason.contains("null until assigned"), "reason " + reason);
        check(byNameOrNull(list(doc.get("types")), "Book") == null, "Book was read with a zero value where Groovy holds null");
    }

    private void everyRefusalBeginsWithACategory() throws Exception {
        List<String> bad = new ArrayList<>();
        try (Stream<Path> dirs = Files.list(testdata.resolve("refuse"))) {
            for (Path d : dirs.sorted().toList()) {
                Map<String, Object> doc = Reader.read(d);
                for (Object r : list(doc.get("refusals"))) {
                    String reason = (String) map(r).get("reason");
                    int colon = reason.indexOf(": ");
                    if (colon < 0 || !CATEGORIES.contains(reason.substring(0, colon))) {
                        bad.add(d.getFileName() + " " + map(r).get("pos") + " " + reason);
                    }
                }
                for (String reason : foreignReasons(doc.get("funcs"))) {
                    int colon = reason.indexOf(": ");
                    if (colon < 0 || !CATEGORIES.contains(reason.substring(0, colon))) {
                        bad.add(d.getFileName() + " foreign " + reason);
                    }
                }
            }
        }
        check(bad.isEmpty(), "reasons without a category: " + bad);
    }

    /** README.md has a {@code ### `<category>`} section for each category, each holding a {@code groovy} example. */
    private void theReadmeListsTheCategories() throws Exception {
        String readme = Files.readString(root.resolve("adapters/groovy/README.md"));
        List<String> missing = new ArrayList<>();
        for (String c : CATEGORIES) {
            String head = "### `" + c + "`";
            int at = readme.indexOf(head);
            if (at < 0) {
                missing.add(c + " (no section)");
                continue;
            }
            int next = readme.indexOf("\n#", at + head.length());
            String section = readme.substring(at, next < 0 ? readme.length() : next);
            if (!section.contains("```groovy")) {
                missing.add(c + " (no example)");
            }
        }
        check(missing.isEmpty(), "README.md lacks " + missing);
    }

    // ---- Grails: the lending application ----

    /** A case that cannot run here; the summary counts it as skipped, saying why. */
    static final class Skip extends RuntimeException {
        private static final long serialVersionUID = 1L;

        Skip(String why) {
            super(why, null, false, false);
        }
    }

    /**
     * The Grails 2.5.6 distribution jars a Grails application is read with, relative to {@code
     * GRAILS_HOME}: {@code grails.transaction.Transactional} and its transform (grails-core) and
     * the Spring enums its members name (spring-tx). The transform is not run, so the jars it
     * loads classes from as it runs are not among them. README.md names each.
     */
    static final List<String> GRAILS_JARS = List.of(
        "dist/grails-core-2.5.6.jar",
        "lib/org.springframework/spring-tx/jars/spring-tx-4.1.9.RELEASE.jar");

    /** The identifiers {@code TransactionalTransform} introduces, which no read may hold. */
    static final List<String> TRANSFORM_IDENTIFIERS = List.of(
        "$tt__", "transactionManager", "transactionStatus", "GrailsTransactionTemplate", "PlatformTransactionManager");

    private static final String AUDIT = "grails-app/services/lending/TransactionalAuditService.groovy";
    private static final String LOAN_SERVICE = "grails-app/services/lending/LoanService.groovy";

    private Path lendingDir() {
        return testdata.resolve("lending");
    }

    private Map<String, Object> lending() throws Exception {
        return Main.read(lendingDir(), List.of());
    }

    private List<Path> grailsJars() {
        String home = System.getenv("GRAILS_HOME");
        if (home == null || home.isEmpty()) {
            throw new Skip("GRAILS_HOME is not set; set it to a Grails 2.5.6 distribution to run this case");
        }
        List<Path> out = new ArrayList<>();
        for (String j : GRAILS_JARS) {
            Path p = Path.of(home).resolve(j);
            check(Files.isRegularFile(p), "GRAILS_HOME has no " + j);
            out.add(p);
        }
        return out;
    }

    private void lendingMatchesExpected() throws Exception {
        List<String> problems = Lending.compare(lending(), lendingDir());
        check(problems.isEmpty(), "the lending read differs from the fixture:\n  " + String.join("\n  ", problems));
    }

    private void lendingReadTwiceIsCanonical() throws Exception {
        String a = Json.write(lending());
        String b = Json.write(lending());
        check(a.equals(b), "two reads of lending differ");
        Path doc = work.resolve("lending.raw.json");
        Files.writeString(doc, a, StandardCharsets.UTF_8);
        Process p = new ProcessBuilder(codespec.toString(), "validate", "--canonical", doc.toString()).inheritIO().start();
        check(p.waitFor() == 0, "codespec validate --canonical failed on the lending read");
    }

    private void lendingEqualsTheCheckedInDocument() throws Exception {
        Path checkedIn = testdata.resolve("lending.read.cir.json");
        check(Files.isRegularFile(checkedIn), "no " + checkedIn);
        check(Json.write(lending()).equals(Files.readString(checkedIn)), "the lending read differs from " + checkedIn
            + "; regenerate it with task examples");
    }

    private void lendingResolveOnlyYieldsNothing() throws Exception {
        Map<String, Object> doc = lending();
        List<String> found = new ArrayList<>();
        for (Map<String, Object> s : statements(doc)) {
            String pos = (String) s.get("pos");
            if (pos != null && (pos.startsWith("grails-app/controllers/") || pos.startsWith("src/")
                || pos.startsWith("grails-app/utils/") || pos.startsWith("grails-app/conf/"))) {
                found.add(pos);
            }
        }
        for (Object r : list(doc.get("refusals"))) {
            String pos = (String) map(r).get("pos");
            if (!pos.startsWith("grails-app/domain/") && !pos.startsWith("grails-app/services/")) {
                found.add(pos);
            }
        }
        for (String name : List.of("ToolController", "ShelfCodes", "DueDates")) {
            if (byNameOrNull(list(doc.get("types")), name) != null) {
                found.add("type " + name);
            }
        }
        check(found.isEmpty(), "read from a resolve-only file: " + found);
        int line = lineOf(lendingDir().resolve(LOAN_SERVICE), "DueDates.dueDay(");
        for (Object r : list(doc.get("refusals"))) {
            Map<String, Object> rm = map(r);
            if ((LOAN_SERVICE + ":" + line).equals(rm.get("pos"))) {
                check(!((String) rm.get("reason")).startsWith("unresolved"), "the DueDates call is refused " + rm.get("reason"));
            }
        }
        check(funcByKeyOrNull(doc, "LoanService.dueDayFor") != null, "LoanService.dueDayFor was not read as a function");
    }

    private void lendingToolNameIsAPointer() throws Exception {
        Map<String, Object> doc = lending();
        String ptrString = "{\"k\":\"ptr\",\"elem\":{\"k\":\"string\"}}";
        for (String[] sf : new String[][] {{"Tool", "name"}, {"Tool", "shelf"}, {"Member", "fullName"}, {"Member", "email"}}) {
            Map<String, Object> s = byName(list(doc.get("types")), sf[0]);
            Object t = byName(list(s.get("fields")), sf[1]).get("t");
            check(same(t, ptrString), sf[0] + "." + sf[1] + " is " + Json.write(t).replaceAll("\\s+", " "));
            Map<String, Object> ctor = funcByKey(doc, "New" + sf[0]);
            Map<String, Object> lit = map(map(list(map(list(ctor.get("body")).get(0)).get("rhs")).get(0)).get("x"));
            check(byNameOrNull(list(lit.get("fields")), sf[1]) == null, "New" + sf[0] + " sets " + sf[1] + ": its zero value is not nil");
        }
    }

    private void lendingGormCallsAreForeign() throws Exception {
        Map<String, Object> doc = lending();
        String catalog = "grails-app/services/lending/CatalogService.groovy";
        foreignWith(doc, catalog + ":" + lineOf(lendingDir().resolve(catalog), "Tool.findByName(n)"), "gorm: findByName");
        foreignWith(doc, LOAN_SERVICE + ":" + lineOf(lendingDir().resolve(LOAN_SERVICE), "loan.save()"), "gorm: save");
    }

    private void foreignWith(Map<String, Object> doc, String pos, String prefix) {
        boolean refusal = false;
        for (Object r : list(doc.get("refusals"))) {
            if (pos.equals(map(r).get("pos")) && ((String) map(r).get("reason")).startsWith(prefix)) {
                refusal = true;
            }
        }
        check(refusal, "no refusal starting " + prefix + " at " + pos);
        boolean foreign = false;
        for (Map<String, Object> f : foreigns(doc.get("funcs"))) {
            if (pos.equals(f.get("pos")) && ((String) f.get("reason")).startsWith(prefix)) {
                foreign = true;
            }
        }
        check(foreign, "no foreign statement starting " + prefix + " at " + pos);
    }

    private void lendingSpringInjection() throws Exception {
        Map<String, Object> doc = lending();
        Path src = lendingDir().resolve(LOAN_SERVICE);
        for (String field : List.of("def catalogService", "FeeService feeService")) {
            String pos = LOAN_SERVICE + ":" + lineOf(src, field);
            check(reasonsAt(doc, pos).equals(List.of("spring: injected dependency")), pos + " " + reasonsAt(doc, pos));
        }
        foreignWith(doc, LOAN_SERVICE + ":" + lineOf(src, "catalogService.toolById"), "spring: call through an injected dependency");
        foreignWith(doc, LOAN_SERVICE + ":" + lineOf(src, "feeService.lateFeeCents"), "spring: call through an injected dependency");
        Map<String, Object> s = byName(list(doc.get("types")), "LoanService");
        check(list(s.get("fields")).isEmpty(), "LoanService holds the injected fields: " + Json.write(s.get("fields")));
    }

    private void lendingDomainMetadata() throws Exception {
        Map<String, Object> doc = lending();
        String[][] want = {
            {"grails-app/domain/lending/Loan.groovy", "static belongsTo", "belongsTo"},
            {"grails-app/domain/lending/Loan.groovy", "static mapping", "mapping"},
            {"grails-app/domain/lending/Member.groovy", "static hasMany", "hasMany"},
            {"grails-app/domain/lending/Member.groovy", "static constraints", "constraints"},
            {"grails-app/domain/lending/Tool.groovy", "static hasMany", "hasMany"},
            {"grails-app/domain/lending/Tool.groovy", "static constraints", "constraints"},
            {"grails-app/domain/lending/Tool.groovy", "static mapping", "mapping"},
        };
        for (String[] w : want) {
            String pos = w[0] + ":" + lineOf(lendingDir().resolve(w[0]), w[1]);
            check(reasonsAt(doc, pos).equals(List.of("gorm: mapping metadata " + w[2])), pos + " " + reasonsAt(doc, pos));
        }
    }

    private static List<String> reasonsAt(Map<String, Object> doc, String pos) {
        List<String> out = new ArrayList<>();
        for (Object r : list(doc.get("refusals"))) {
            if (pos.equals(map(r).get("pos"))) {
                out.add((String) map(r).get("reason"));
            }
        }
        return out;
    }

    private void lendingTransactionalUnresolvedWithoutJars() throws Exception {
        Map<String, Object> doc = lending();
        String pos = AUDIT + ":" + lineOf(lendingDir().resolve(AUDIT), "import grails.transaction.Transactional");
        check(reasonsAt(doc, pos).equals(List.of("unresolved: grails.transaction.Transactional")), pos + " " + reasonsAt(doc, pos));
        for (Object r : list(doc.get("refusals"))) {
            String p = (String) map(r).get("pos");
            check(!p.startsWith(AUDIT) || p.equals(pos), "a second refusal in the refused artefact: " + p);
        }
        check(byNameOrNull(list(doc.get("types")), "TransactionalAuditService") == null, "TransactionalAuditService was read");
        for (String key : List.of("CatalogService.doubled", "FeeService.receipt", "Member.greeting")) {
            check(funcByKeyOrNull(doc, key) != null, key + " was not read beside the refused artefact");
        }
    }

    private void lendingEveryReasonHasACategory() throws Exception {
        Map<String, Object> doc = lending();
        List<String> bad = new ArrayList<>();
        List<String> reasons = new ArrayList<>(foreignReasons(doc.get("funcs")));
        for (Object r : list(doc.get("refusals"))) {
            reasons.add((String) map(r).get("reason"));
        }
        for (String reason : reasons) {
            int colon = reason.indexOf(": ");
            if (colon < 0 || !CATEGORIES.contains(reason.substring(0, colon))) {
                bad.add(reason);
            }
        }
        check(bad.isEmpty(), "reasons without a category: " + bad);
    }

    private void plainDirectoryReadsAsGroovy() throws Exception {
        Map<String, Object> doc = Main.read(testdata.resolve("typed"), List.of());
        check(!doc.containsKey("refusals"), "refusals: " + Json.write(doc.getOrDefault("refusals", List.of())));
        check(Json.write(doc).equals(Json.write(typed())), "Main.read of typed differs from the plain read");
    }

    private void duplicateSimpleNamesAreRefused() throws Exception {
        Path app = work.resolve("duplicate-app");
        write(app.resolve("application.properties"), "app.grails.version=2.5.6\napp.name=dup\n");
        write(app.resolve("grails-app/domain/a/Book.groovy"), "package a\n\nclass Book {\n    int pages = 0\n}\n");
        write(app.resolve("grails-app/domain/b/Book.groovy"), "package b\n\nclass Book {\n    int copies = 0\n}\n");
        write(app.resolve("grails-app/services/a/ShelfService.groovy"),
            "package a\n\nclass ShelfService {\n    int one() {\n        return 1\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        check(reasonsAt(doc, "grails-app/domain/a/Book.groovy:3").equals(List.of("unsupported: duplicate simple name Book")),
            "a/Book " + reasonsAt(doc, "grails-app/domain/a/Book.groovy:3"));
        check(reasonsAt(doc, "grails-app/domain/b/Book.groovy:3").equals(List.of("unsupported: duplicate simple name Book")),
            "b/Book " + reasonsAt(doc, "grails-app/domain/b/Book.groovy:3"));
        check(byNameOrNull(list(doc.get("types")), "Book") == null, "a Book was read");
        check(funcByKeyOrNull(doc, "ShelfService.one") != null, "ShelfService.one was not read");
        check("dup".equals(doc.get("name")), "module name " + doc.get("name"));
    }

    /**
     * Groovy lets a method name be any quoted string; cir/1 names are identifiers, so the method
     * is a refusal entry, the method beside it is read, and the document passes strict reading.
     */
    private void aQuotedMethodNameIsRefused() throws Exception {
        Path dir = work.resolve("quoted-method-name");
        write(dir.resolve("Calc.groovy"), "package calc\n\nimport groovy.transform.CompileStatic\n\n@CompileStatic\n"
            + "class Calc {\n    long \"add two\"(long a, long b) {\n        return a + b\n    }\n\n"
            + "    long plus(long a, long b) {\n        return a + b\n    }\n}\n");
        Map<String, Object> doc = Reader.read(dir);
        check(funcByKeyOrNull(doc, "Calc.add two") == null, "the method \"add two\" was read as a function");
        check(funcByKeyOrNull(doc, "Calc.plus") != null, "Calc.plus, beside it, was not read");
        check(reasonsAt(doc, "Calc.groovy:7").size() == 1, "refusals " + Json.write(doc.getOrDefault("refusals", List.of())));
        Path out = work.resolve("quoted-method-name.json");
        Files.writeString(out, Json.write(doc), StandardCharsets.UTF_8);
        Process p = new ProcessBuilder(codespec.toString(), "validate", out.toString()).inheritIO().start();
        check(p.waitFor() == 0, "codespec validate refused the read");
    }

    private static void write(Path p, String text) throws IOException {
        Files.createDirectories(p.getParent());
        Files.writeString(p, text, StandardCharsets.UTF_8);
    }

    private void theReadmeNamesTheGrailsJars() throws Exception {
        String readme = Files.readString(root.resolve("adapters/groovy/README.md"));
        for (String j : GRAILS_JARS) {
            check(readme.contains("$GRAILS_HOME/" + j), "README.md does not name $GRAILS_HOME/" + j);
        }
        check(readme.contains("CODESPEC_GROOVY_CLASSPATH"), "README.md does not name CODESPEC_GROOVY_CLASSPATH");
        check(!readme.contains("/home/"), "README.md names a home directory");
    }

    private void lendingTransactionalWithJars() throws Exception {
        List<Path> jars = grailsJars();
        Map<String, Object> doc = Main.read(lendingDir(), jars);
        for (Object r : list(doc.get("refusals"))) {
            Map<String, Object> rm = map(r);
            check(!((String) rm.get("reason")).startsWith("unresolved"), "with the Grails jars: " + rm.get("pos") + " " + rm.get("reason"));
        }
        check(byNameOrNull(list(doc.get("types")), "TransactionalAuditService") != null, "TransactionalAuditService was not read");
        List<String> lines = Files.readAllLines(lendingDir().resolve(AUDIT));
        for (int i = 0; i < lines.size(); i++) {
            String method = Lending.methodName(lines.get(i));
            if (method == null) {
                continue;
            }
            String pos = AUDIT + ":" + (i + 1);
            Map<String, Object> fn = funcByKeyOrNull(doc, "TransactionalAuditService." + method);
            List<String> reasons = reasonsAt(doc, pos);
            boolean readFromSource = fn != null && pos.equals(fn.get("pos")) && reasons.isEmpty() && withinMethod(fn, lines, i + 1);
            check(readFromSource, method + " at " + pos + " is not read from its source body, the transformation not being run: "
                + (fn == null ? "no function" : Json.write(fn).replaceAll("\\s+", " ")) + " " + reasons);
        }
        List<String> others = new ArrayList<>();
        for (String p : Lending.compare(doc, lendingDir())) {
            if (!p.startsWith(AUDIT)) {
                others.add(p);
            }
        }
        check(others.isEmpty(), "with the Grails jars the other artefacts differ:\n  " + String.join("\n  ", others));
    }

    /** Whether every statement of a function lies in the source lines of the method declared at {@code line}. */
    private static boolean withinMethod(Map<String, Object> fn, List<String> lines, int line) {
        int indent = lines.get(line - 1).length() - lines.get(line - 1).stripLeading().length();
        int end = line;
        while (end < lines.size()) {
            String l = lines.get(end);
            end++;
            if (l.stripLeading().startsWith("}") && l.length() - l.stripLeading().length() == indent) {
                break;
            }
        }
        for (Map<String, Object> s : statements(fn.get("body"))) {
            int at = Lending.lineOf((String) s.get("pos"));
            if (at <= line || at > end) {
                return false;
            }
        }
        return true;
    }

    private void lendingNoTransformIdentifier() throws Exception {
        List<Path> jars = grailsJars();
        Map<String, Object> doc = Main.read(lendingDir(), jars);
        List<String> found = new ArrayList<>();
        holds(doc, "(module)", found);
        check(found.isEmpty(), "identifiers of the transactional transform in the read: " + found);
    }

    /** Every identifier of {@link #TRANSFORM_IDENTIFIERS} in a string under {@code o}, with the nearest enclosing position. */
    private static void holds(Object o, String pos, List<String> found) {
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            String here = m.get("pos") instanceof String ? (String) m.get("pos") : pos;
            for (Map.Entry<String, Object> e : m.entrySet()) {
                holds(e.getKey(), here, found);
                holds(e.getValue(), here, found);
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                holds(v, pos, found);
            }
        } else if (o instanceof String) {
            for (String id : TRANSFORM_IDENTIFIERS) {
                if (((String) o).contains(id)) {
                    found.add(id + " at " + pos);
                }
            }
        }
    }

    // ---- adversary: story:grails-artefacts ----

    /** A Grails 2.5.6 application named {@code library} under the work dir, emptied first. */
    private Path adversaryApp(String name) throws IOException {
        Path app = work.resolve("adversary-grails/" + name);
        if (Files.exists(app)) {
            try (Stream<Path> s = Files.walk(app)) {
                for (Path p : s.sorted(java.util.Comparator.reverseOrder()).toList()) {
                    Files.delete(p);
                }
            }
        }
        write(app.resolve("application.properties"), "app.grails.version=2.5.6\napp.name=library\n");
        return app;
    }

    /**
     * Grails autowires every service by name (ServicesGrailsPlugin: {@code bean.autowire = true},
     * {@code innerBean.autowire = "byName"}), and {@code grailsApplication} is always a bean. {@code
     * def grailsApplication} is the idiomatic Grails 2 way to reach the configuration; the reader
     * refuses it as {@code dynamic} and with it the whole service, so {@code one} is lost.
     */
    private void adversaryGrailsApplicationInjected() throws Exception {
        Path app = adversaryApp("grails-application");
        String svc = "grails-app/services/lib/AppService.groovy";
        write(app.resolve(svc), "package lib\n\nclass AppService {\n\n    def grailsApplication\n\n"
            + "    int one() {\n        return 1\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        check(funcByKeyOrNull(doc, "AppService.one") != null, "AppService.one is not read; refusals: " + reasonsAt(doc, svc + ":5"));
        check(reasonsAt(doc, svc + ":5").equals(List.of("spring: injected dependency")), "def grailsApplication at " + svc
            + ":5 is " + reasonsAt(doc, svc + ":5"));
    }

    /**
     * story:grails-artefacts wires an uninitialised domain property to the shared ptr(T) rule of
     * story:nullable-reference-fields; the Java reader reads a write of such a field and its
     * comparison with null with no refusal. The Grails layer refuses every read and write of it.
     */
    private void adversaryNullablePropertyUses() throws Exception {
        Path app = adversaryApp("nullable-uses");
        write(app.resolve("grails-app/domain/lib/Book.groovy"), "package lib\n\nclass Book {\n\n    String title\n"
            + "    int pages = 0\n\n"
            + "    void rename(String t) {\n        title = t\n    }\n\n"
            + "    boolean untitled() {\n        return title == null\n    }\n\n"
            + "    String titleOr(String fallback) {\n        if (title == null) {\n            return fallback\n        }\n"
            + "        return title\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        // Coordinator decision (story:grails-artefacts, round 3): the shared ptr(T) rule holds, so
        // titleOr is refused exactly at its `return title`, as the Java reader refuses it; the
        // other two methods read with no refusal.
        String book = "grails-app/domain/lib/Book.groovy";
        int ret = Files.readAllLines(app.resolve(book)).stream().map(String::trim).toList().indexOf("return title") + 1;
        Map<String, Object> titleOr = funcByKeyOrNull(doc, "Book.titleOr");
        check(titleOr != null, "Book.titleOr is not read as a function");
        List<String> at = new ArrayList<>();
        for (Map<String, Object> f : foreigns(titleOr)) {
            at.add(f.get("pos") + " " + f.get("reason"));
        }
        check(at.size() == 1 && at.get(0).startsWith(book + ":" + ret + " unsupported: property title, null until set, returned"),
            "Book.titleOr is not refused exactly at its return title (" + book + ":" + ret + "): " + at);
        check(reasonsAt(doc, book + ":" + ret).size() == 1, "refusals at " + book + ":" + ret + ": " + reasonsAt(doc, book + ":" + ret));
        List<String> refused = new ArrayList<>();
        for (String key : List.of("Book.rename", "Book.untitled")) {
            Map<String, Object> fn = funcByKeyOrNull(doc, key);
            if (fn == null) {
                refused.add(key + ": not read");
            } else if (!foreigns(fn).isEmpty()) {
                refused.add(key + ": " + foreignReasons(fn));
            }
        }
        check(refused.isEmpty(), "uses of the ptr(string) property title refused: " + refused);
    }

    /**
     * story:grails-artefacts, Context: "All of grails-app/ and src/ (src/groovy, src/java) go to
     * the compiler, so a service that references a class from ... src/ resolves." A src/java class
     * named in a service signature refuses the whole service as unresolved.
     */
    private void adversarySrcJavaResolves() throws Exception {
        Path app = adversaryApp("src-java");
        write(app.resolve("src/java/lib/JavaMath.java"), "package lib;\n\npublic final class JavaMath {\n"
            + "    private JavaMath() {}\n\n    public static int twice(int n) {\n        return n * 2;\n    }\n}\n");
        String svc = "grails-app/services/lib/JavaUserService.groovy";
        write(app.resolve(svc), "package lib\n\nclass JavaUserService {\n\n"
            + "    int viaSrcJava(JavaMath m, int n) {\n        return n\n    }\n\n"
            + "    int pure(int n) {\n        return n + 2\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        for (Object r : list(doc.get("refusals"))) {
            check(!((String) map(r).get("reason")).startsWith("unresolved"), map(r).get("pos") + " " + map(r).get("reason"));
        }
        check(funcByKeyOrNull(doc, "JavaUserService.pure") != null, "JavaUserService.pure is not read");
    }

    /**
     * Grails 2 lets a class in src/groovy be a GORM entity through {@code @grails.persistence.Entity}.
     * Run, its local transformation needs spring-context, and the NoClassDefFoundError failed the
     * whole read, though src/groovy is resolve-only and "yields no struct, function or refusal".
     * The transformation is of a jar and is not run: with the jars README.md names the read
     * succeeds and the domain class is read.
     */
    private void adversaryEntityInSrcGroovy() throws Exception {
        List<Path> jars = grailsJars();
        Path app = adversaryApp("entity-src-groovy");
        write(app.resolve("src/groovy/lib/Record.groovy"), "package lib\n\nimport grails.persistence.Entity\n\n@Entity\n"
            + "class Record {\n\n    int pages = 0\n}\n");
        write(app.resolve("grails-app/domain/lib/Shelf.groovy"), "package lib\n\nclass Shelf {\n\n    int slots = 0\n\n"
            + "    int more(int n) {\n        return slots + n\n    }\n}\n");
        Map<String, Object> doc;
        try {
            doc = Main.read(app, jars);
        } catch (Throwable e) {
            throw new AssertionError("the read failed: " + e);
        }
        check(funcByKeyOrNull(doc, "Shelf.more") != null, "Shelf.more is not read");
    }

    /**
     * Spring's autowire by name sets a service property whatever its initializer, so in Grails
     * {@code countService} is the shared singleton, not {@code new CountService()}. Run with
     * spring-beans 4.1.9 from the Grails 2.5.6 distribution: twice() gives 3 after one earlier bump
     * of the bean, 2 on a fresh instance. The reader reads twice() with no refusal on its own instance.
     */
    private void adversaryInitialisedServiceField() throws Exception {
        Path app = adversaryApp("initialised-service-field");
        write(app.resolve("grails-app/services/lib/CountService.groovy"), "package lib\n\nclass CountService {\n\n"
            + "    int calls = 0\n\n    int bump() {\n        calls = calls + 1\n        return calls\n    }\n}\n");
        String svc = "grails-app/services/lib/UserService.groovy";
        write(app.resolve(svc), "package lib\n\nclass UserService {\n\n    CountService countService = new CountService()\n\n"
            + "    int twice() {\n        countService.bump()\n        return countService.bump()\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        Map<String, Object> fn = funcByKeyOrNull(doc, "UserService.twice");
        check(fn == null || !foreigns(fn).isEmpty() || !reasonsAt(doc, svc + ":5").isEmpty(),
            "UserService.twice is read as calling bump on the service's own CountService, with no refusal");
    }

    /**
     * A method-level {@code @Transactional} on {@code size(int)} would rewrite only that method.
     * Its overload {@code size(String)} was once matched by name and parameter count, emptied and
     * refused as rewritten, which it was not, losing its own reason, overloading. The
     * transformation is not run and no method is refused as rewritten.
     */
    private void adversaryOverloadNotRewritten() throws Exception {
        List<Path> jars = grailsJars();
        Path app = adversaryApp("overload-transform");
        String svc = "grails-app/services/lib/OverloadTxService.groovy";
        write(app.resolve(svc), "package lib\n\nimport grails.transaction.Transactional\n\nclass OverloadTxService {\n\n"
            + "    @Transactional\n    int size(int n) {\n        return n\n    }\n\n"
            + "    int size(String s) {\n        return s.length()\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, jars);
        int line = lineOf(app.resolve(svc), "int size(String s)");
        for (String reason : reasonsAt(doc, svc + ":" + line)) {
            check(!reason.startsWith("unsupported: grails transform"), svc + ":" + line + " " + reason);
        }
    }

    /** {@code listOrderBy*} is a GORM dynamic method (Grails 2.5 reference, Domain Classes). */
    private void adversaryListOrderBy() throws Exception {
        Path app = adversaryApp("list-order-by");
        write(app.resolve("grails-app/domain/lib/Book.groovy"), "package lib\n\nclass Book {\n\n    int pages = 0\n}\n");
        String svc = "grails-app/services/lib/ShelfService.groovy";
        write(app.resolve(svc), "package lib\n\nclass ShelfService {\n\n"
            + "    List<Book> ordered() {\n        return Book.listOrderByPages()\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        foreignWith(doc, svc + ":6", "gorm: listOrderByPages");
    }

    // ---- a class, a jar or a file that does not resolve refuses a file; it never ends the read ----

    private static final String PLAIN_SERVICE = "grails-app/services/lib/PlainService.groovy";

    /** An application under the work dir holding one service that needs nothing: every case requires it read. */
    private Path appWithPlainService(String name) throws IOException {
        Path app = adversaryApp(name);
        write(app.resolve(PLAIN_SERVICE), "package lib\n\nclass PlainService {\n\n"
            + "    int pure(int n) {\n        return n + 2\n    }\n}\n");
        return app;
    }

    /** Read {@code app} with {@code jars}; whatever the read throws fails the case, naming it. */
    private static Map<String, Object> readWritesADocument(Path app, List<Path> jars) {
        Map<String, Object> doc;
        try {
            doc = Main.read(app, jars);
        } catch (Throwable e) {
            throw new AssertionError("the read ended without a document: " + e);
        }
        check(funcByKeyOrNull(doc, "PlainService.pure") != null, "PlainService.pure, which needs nothing, is not read");
        return doc;
    }

    /** {@code file} is refused as a whole, once, at {@code line}, its reason beginning {@code reason}; {@code key} is not read. */
    private static void refusedAsAWhole(Map<String, Object> doc, String file, int line, String reason, String key) {
        List<String> all = new ArrayList<>();
        for (Object r : list(doc.get("refusals"))) {
            String pos = (String) map(r).get("pos");
            if (pos.startsWith(file + ":")) {
                all.add(pos + " " + map(r).get("reason"));
            }
        }
        check(all.size() == 1 && all.get(0).startsWith(file + ":" + line + " " + reason),
            file + " is not refused once at line " + line + " as " + reason + ": " + all);
        check(funcByKeyOrNull(doc, key) == null, key + ", of a refused file, is read");
    }

    /**
     * Compile {@code sources} (binary class name to Java text) with javac against {@code classpath}
     * and pack every class file whose entry name {@code keep} accepts, after {@code edit}, into a jar
     * under the work dir.
     */
    private Path jarOf(String name, Map<String, String> sources, List<Path> classpath,
        java.util.function.Predicate<String> keep, java.util.function.BiFunction<String, byte[], byte[]> edit) throws IOException {
        Path dir = work.resolve("jars/" + name);
        if (Files.exists(dir)) {
            try (Stream<Path> s = Files.walk(dir)) {
                for (Path p : s.sorted(java.util.Comparator.reverseOrder()).toList()) {
                    Files.delete(p);
                }
            }
        }
        Path src = dir.resolve("src");
        Path classes = dir.resolve("classes");
        Files.createDirectories(classes);
        List<java.io.File> files = new ArrayList<>();
        for (Map.Entry<String, String> e : sources.entrySet()) {
            Path f = src.resolve(e.getKey().replace('.', '/') + ".java");
            write(f, e.getValue());
            files.add(f.toFile());
        }
        javax.tools.JavaCompiler javac = javax.tools.ToolProvider.getSystemJavaCompiler();
        check(javac != null, "no Java compiler in this runtime");
        List<String> cp = new ArrayList<>();
        for (Path p : classpath) {
            cp.add(p.toString());
        }
        List<String> options = List.of("-proc:none", "-nowarn", "--release", "8", "-Xlint:-options", "-d", classes.toString(),
            "-classpath", String.join(java.io.File.pathSeparator, cp));
        java.io.StringWriter out = new java.io.StringWriter();
        try (javax.tools.StandardJavaFileManager fm = javac.getStandardFileManager(null, null, StandardCharsets.UTF_8)) {
            boolean ok = javac.getTask(out, fm, null, options, null, fm.getJavaFileObjectsFromFiles(files)).call();
            check(ok, "javac failed on the classes of " + name + ": " + out);
        }
        Path jar = dir.resolve(name + ".jar");
        try (Stream<Path> s = Files.walk(classes);
            java.util.jar.JarOutputStream j = new java.util.jar.JarOutputStream(Files.newOutputStream(jar))) {
            for (Path p : s.filter(Files::isRegularFile).sorted().toList()) {
                String entry = classes.relativize(p).toString().replace(java.io.File.separatorChar, '/');
                if (keep.test(entry)) {
                    j.putNextEntry(new java.util.jar.JarEntry(entry));
                    j.write(edit.apply(entry, Files.readAllBytes(p)));
                    j.closeEntry();
                }
            }
        }
        return jar;
    }

    private Path jarOf(String name, Map<String, String> sources, java.util.function.Predicate<String> keep) throws IOException {
        return jarOf(name, sources, List.of(), keep, (entry, bytes) -> bytes);
    }

    private static int u2(byte[] b, int at) {
        return (b[at] & 0xff) << 8 | b[at + 1] & 0xff;
    }

    private static int u4(byte[] b, int at) {
        return u2(b, at) << 16 | u2(b, at + 2);
    }

    /**
     * A class file as an API-only stub jar ships it: {@code ACC_ABSTRACT} cleared on the class and on
     * each method, so that a method compiled abstract is a concrete method without a {@code Code}
     * attribute. The JVM refuses to define it, {@code ClassFormatError: Absent Code attribute}.
     */
    private static byte[] withoutCode(byte[] classFile) {
        byte[] b = classFile.clone();
        int p = 10;
        for (int i = 1, n = u2(b, 8); i < n; i++) {
            int tag = b[p] & 0xff;
            if (tag == 1) {
                p += 3 + u2(b, p + 1);
            } else if (tag == 5 || tag == 6) {
                p += 9;
                i++;
            } else if (tag == 7 || tag == 8 || tag == 16 || tag == 19 || tag == 20) {
                p += 3;
            } else if (tag == 15) {
                p += 4;
            } else {
                p += 5;
            }
        }
        b[p] &= ~0x04;
        p += 6;
        p += 2 + 2 * u2(b, p);
        for (int members = 0; members < 2; members++) {
            int count = u2(b, p);
            p += 2;
            for (int i = 0; i < count; i++) {
                if (members == 1) {
                    b[p] &= ~0x04;
                }
                int attributes = u2(b, p + 6);
                p += 8;
                for (int a = 0; a < attributes; a++) {
                    p += 6 + u4(b, p + 2);
                }
            }
        }
        return b;
    }

    /**
     * {@code src/java} is compiled "against the Groovy jar and CODESPEC_GROOVY_CLASSPATH" (README.md):
     * a Java class there that extends a class of such a jar compiles, and must then be defined where
     * that jar is visible, so that the service naming it resolves.
     */
    private void aSrcJavaClassExtendingAClasspathClassResolves() throws Exception {
        Path jar = jarOf("base", Map.of("base.Base", "package base;\n\npublic class Base {\n"
            + "    public int base() {\n        return 1;\n    }\n}\n"), e -> true);
        Path app = appWithPlainService("src-java-extends-classpath");
        write(app.resolve("src/java/lib/Derived.java"), "package lib;\n\npublic class Derived extends base.Base {\n}\n");
        String svc = "grails-app/services/lib/DerivedService.groovy";
        write(app.resolve(svc), "package lib\n\nclass DerivedService {\n\n"
            + "    int viaDerived(Derived d, int n) {\n        return n\n    }\n\n"
            + "    int beside(int n) {\n        return n + 1\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        // lib.Derived resolves: the service is not refused as a whole, and the one method that
        // takes a Derived is refused for what it is, a class that is not read.
        check(funcByKeyOrNull(doc, "DerivedService.beside") != null, "DerivedService.beside is not read: " + Json.write(doc.get("refusals")));
        check(reasonsAt(doc, svc + ":5").equals(List.of("type: type lib.Derived is outside the reader's types")),
            "viaDerived at " + svc + ":5 is " + reasonsAt(doc, svc + ":5"));
        check(list(doc.get("refusals")).size() == 1, "refusals " + Json.write(doc.get("refusals")));
    }

    /**
     * A class of a jar whose own dependency is on no jar: the class is found, and the JVM throws
     * {@code NoClassDefFoundError} for the dependency when Groovy reflects on it. The service that
     * imports it does not resolve and is refused at its import; the other service is read.
     */
    private void aClassWhoseDependencyIsMissingRefusesTheFileNamingIt() throws Exception {
        Map<String, String> sources = new LinkedHashMap<>();
        sources.put("gone.Log", "package gone;\n\npublic class Log {\n}\n");
        sources.put("dep.Helper", "package dep;\n\npublic class Helper {\n    public static gone.Log log;\n\n"
            + "    public static int one() {\n        return 1;\n    }\n}\n");
        Path jar = jarOf("dep", sources, e -> !e.startsWith("gone/"));
        Path app = appWithPlainService("missing-dependency");
        String svc = "grails-app/services/lib/HelperService.groovy";
        write(app.resolve(svc), "package lib\n\nimport dep.Helper\n\nclass HelperService {\n\n"
            + "    int one() {\n        return Helper.one()\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        refusedAsAWhole(doc, svc, 3, "unresolved: dep.Helper", "HelperService.one");
        check(reasonsAt(doc, svc + ":3").get(0).contains("gone/Log"), "the reason does not name the missing class gone/Log: "
            + reasonsAt(doc, svc + ":3"));
    }

    /**
     * A class no file names, reached as the result of a call, whose own dependency is on no jar:
     * the type checker meets the {@code NoClassDefFoundError} after the compilation. No line names
     * the class, so the service that reaches it is refused at its class; the service refused for
     * its import before that stays refused once, and the other service is read.
     */
    private void aClassReachedThroughACallWhoseDependencyIsMissingRefusesTheFileReachingIt() throws Exception {
        Map<String, String> sources = new LinkedHashMap<>();
        sources.put("gone.Log", "package gone;\n\npublic class Log {\n}\n");
        sources.put("dep.Inner", "package dep;\n\npublic class Inner {\n    public static gone.Log log;\n\n"
            + "    public int size() {\n        return 0;\n    }\n}\n");
        sources.put("dep.Outer", "package dep;\n\npublic class Outer {\n"
            + "    public static Inner inner() {\n        return new Inner();\n    }\n}\n");
        Path jar = jarOf("reach", sources, e -> !e.startsWith("gone/"));
        Path app = appWithPlainService("missing-dependency-reached");
        String svc = "grails-app/services/lib/ReachService.groovy";
        write(app.resolve(svc), "package lib\n\nimport dep.Outer\n\nclass ReachService {\n\n"
            + "    int size() {\n        return Outer.inner().size()\n    }\n}\n");
        String board = "grails-app/services/lib/BoardService.groovy";
        write(app.resolve(board), "package lib\n\nimport gone.Board\n\nclass BoardService {\n\n"
            + "    int one() {\n        return 1\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        refusedAsAWhole(doc, svc, 5, "unresolved: ", "ReachService.size");
        check(reasonsAt(doc, svc + ":5").get(0).contains("dep.Inner") && reasonsAt(doc, svc + ":5").get(0).contains("gone/Log"),
            "the reason does not name dep.Inner and the missing class gone/Log: " + reasonsAt(doc, svc + ":5"));
        refusedAsAWhole(doc, board, 3, "unresolved: gone.Board", "BoardService.one");
    }

    /**
     * Two local AST transformations of a jar that need a class on no jar, as Grails's
     * transformations need the classes of other jars. The reader runs no transformation of a jar,
     * and Groovy's compiler still loads the class of each to see that it is one. {@code
     * MarkTransform} extends the missing class, so it cannot be loaded: the class its annotation is
     * on is refused at the annotation's line, naming the missing class. {@code TouchTransform}
     * needs the missing class only as it runs: it is loaded and not run, its annotation is a mark
     * and the class that carries it is read. The other service is read.
     */
    private void aTransformationThatNeedsAMissingClassRefusesTheClassItAnnotates() throws Exception {
        Map<String, String> sources = new LinkedHashMap<>();
        sources.put("gone.Log", "package gone;\n\npublic class Log {\n    public static void touch() {\n    }\n}\n");
        for (String name : List.of("Mark", "Touch")) {
            sources.put("xf." + name, "package xf;\n\n@java.lang.annotation.Retention(java.lang.annotation.RetentionPolicy.SOURCE)\n"
                + "@java.lang.annotation.Target(java.lang.annotation.ElementType.TYPE)\n"
                + "@org.codehaus.groovy.transform.GroovyASTTransformationClass(\"xf." + name + "Transform\")\n"
                + "public @interface " + name + " {\n}\n");
        }
        sources.put("xf.MarkTransform", "package xf;\n\n"
            + "@org.codehaus.groovy.transform.GroovyASTTransformation(phase = org.codehaus.groovy.control.CompilePhase.CANONICALIZATION)\n"
            + "public class MarkTransform extends gone.Log implements org.codehaus.groovy.transform.ASTTransformation {\n"
            + "    public void visit(org.codehaus.groovy.ast.ASTNode[] nodes, org.codehaus.groovy.control.SourceUnit source) {\n"
            + "    }\n}\n");
        sources.put("xf.TouchTransform", "package xf;\n\n"
            + "@org.codehaus.groovy.transform.GroovyASTTransformation(phase = org.codehaus.groovy.control.CompilePhase.CANONICALIZATION)\n"
            + "public class TouchTransform implements org.codehaus.groovy.transform.ASTTransformation {\n"
            + "    public void visit(org.codehaus.groovy.ast.ASTNode[] nodes, org.codehaus.groovy.control.SourceUnit source) {\n"
            + "        gone.Log.touch();\n    }\n}\n");
        Path jar = jarOf("xf", sources, List.of(groovyJar()), e -> !e.startsWith("gone/"), (entry, bytes) -> bytes);
        Path app = appWithPlainService("transformation-missing-class");
        String svc = "grails-app/services/lib/MarkedService.groovy";
        write(app.resolve(svc), "package lib\n\nimport xf.Mark\n\n@Mark\nclass MarkedService {\n\n"
            + "    int one() {\n        return 1\n    }\n}\n");
        String touched = "grails-app/services/lib/TouchedService.groovy";
        write(app.resolve(touched), "package lib\n\nimport xf.Touch\n\n@Touch\nclass TouchedService {\n\n"
            + "    int two() {\n        return 2\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        refusedAsAWhole(doc, svc, 5, "unresolved: ", "MarkedService.one");
        check(reasonsAt(doc, svc + ":5").get(0).contains("gone/Log"), "the reason does not name the missing class gone/Log: "
            + reasonsAt(doc, svc + ":5"));
        check(reasonsAt(doc, touched + ":5").equals(List.of("unsupported: annotation xf.Touch: annotations with behaviour are outside cir/1")),
            "@Touch at " + touched + ":5 is " + reasonsAt(doc, touched + ":5"));
        check(funcByKeyOrNull(doc, "TouchedService.two") != null, "TouchedService.two, of the class whose transformation is not run, is"
            + " not read: " + Json.write(doc.get("refusals")));
    }

    /**
     * A stub jar holds API-only class files, methods without a {@code Code} attribute: javac and
     * Groovy's resolver find the class, and the JVM refuses to define it with {@code
     * ClassFormatError}. The service that imports it is refused at its import; the other is read.
     */
    private void aStubClassWithoutCodeRefusesTheFileNamingIt() throws Exception {
        Path jar = jarOf("stub", Map.of("stub.Kind", "package stub;\n\npublic abstract class Kind {\n"
            + "    public abstract int rank();\n}\n"), List.of(), e -> true, (entry, bytes) -> withoutCode(bytes));
        try (java.net.URLClassLoader l = new java.net.URLClassLoader(new java.net.URL[] {jar.toUri().toURL()}, null)) {
            l.loadClass("stub.Kind");
            throw new AssertionError("the stub class the case builds is defined by the JVM");
        } catch (ClassFormatError e) {
            check(String.valueOf(e.getMessage()).contains("Absent Code attribute"), "the stub class fails as " + e);
        }
        Path app = appWithPlainService("stub-jar");
        String svc = "grails-app/services/lib/KindService.groovy";
        write(app.resolve(svc), "package lib\n\nimport stub.Kind\n\nclass KindService {\n\n"
            + "    int rank(Kind k) {\n        return 1\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        refusedAsAWhole(doc, svc, 3, "unresolved: stub.Kind", "KindService.rank");
        check(reasonsAt(doc, svc + ":3").get(0).contains("Absent Code attribute"), "the reason does not say why stub.Kind cannot be loaded: "
            + reasonsAt(doc, svc + ":3"));
    }

    /**
     * An annotation of a jar whose member names a constant of a class on no jar: Groovy reports
     * "unable to find class … for annotation attribute constant" at the annotation. The file is
     * refused there; the other service is read.
     */
    private void anAnnotationConstantOfAMissingClassRefusesItsFile() throws Exception {
        Path jar = jarOf("ann", Map.of("ann.Tag", "package ann;\n\n"
            + "@java.lang.annotation.Retention(java.lang.annotation.RetentionPolicy.RUNTIME)\n"
            + "public @interface Tag {\n    String value();\n}\n"), e -> true);
        Path app = appWithPlainService("annotation-constant");
        String svc = "grails-app/services/lib/TaggedService.groovy";
        write(app.resolve(svc), "package lib\n\nimport ann.Tag\n\n@Tag(ann.Levels.HIGH)\nclass TaggedService {\n\n"
            + "    int one() {\n        return 1\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        refusedAsAWhole(doc, svc, 5, "unresolved: ann.Levels.HIGH", "TaggedService.one");
    }

    /**
     * A resolve-only enum whose import does not resolve is left out. A service that names it by its
     * qualified name in a static method then holds a name Groovy takes for a variable, "Apparent
     * variable 'lib' was found in a static scope": the service is refused at that line.
     */
    private void aQualifiedReferenceToALeftOutEnumRefusesItsFile() throws Exception {
        Path app = appWithPlainService("left-out-enum");
        write(app.resolve("src/groovy/lib/Color.groovy"), "package lib\n\nimport gone.Palette\n\nenum Color {\n    RED, GREEN\n}\n");
        String svc = "grails-app/services/lib/PaintService.groovy";
        write(app.resolve(svc), "package lib\n\nclass PaintService {\n\n"
            + "    static String first() {\n        return lib.Color.RED.name()\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of());
        refusedAsAWhole(doc, svc, 6, "unresolved: lib", "first");
    }

    /**
     * A domain class refused for its import is left out too. A domain class of its package that
     * names it in a static method is refused at that line, each of the two once.
     */
    private void aReferenceToARefusedDomainClassOfItsPackageRefusesItsFile() throws Exception {
        Path app = appWithPlainService("refused-domain-class");
        String shelf = "grails-app/domain/lib/Shelf.groovy";
        write(app.resolve(shelf), "package lib\n\nimport gone.Board\n\nclass Shelf {\n\n    static final int EMPTY = 0\n\n"
            + "    int slots = 0\n}\n");
        String rack = "grails-app/domain/lib/Rack.groovy";
        write(app.resolve(rack), "package lib\n\nclass Rack {\n\n    int slots = 0\n\n"
            + "    static int none() {\n        return Shelf.EMPTY\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of());
        refusedAsAWhole(doc, shelf, 3, "unresolved: gone.Board", "NewShelf");
        refusedAsAWhole(doc, rack, 8, "unresolved: Shelf", "none");
    }

    // ---- the general rule: whatever fails in one file refuses that file; it never ends the read ----

    /** The jar Groovy itself is loaded from: what a transformation or a processor of a made-up jar is compiled against. */
    private static Path groovyJar() {
        try {
            return Path.of(groovy.lang.GroovyObject.class.getProtectionDomain().getCodeSource().getLocation().toURI());
        } catch (java.net.URISyntaxException e) {
            throw new AssertionError(e);
        }
    }

    /** The case's premise: this JVM cannot load {@code cls}. */
    private static void notLoadable(String cls) {
        try {
            Class.forName(cls, false, ReaderTest.class.getClassLoader());
        } catch (ClassNotFoundException | LinkageError e) {
            return;
        }
        throw new Skip("this runtime loads " + cls + ", which the case needs to be on no jar");
    }

    /** No refusal names a file under {@code dir}: a resolve-only file yields nothing, left out or not. */
    private static void nothingFrom(Map<String, Object> doc, String dir) {
        for (Object r : list(doc.get("refusals"))) {
            String pos = (String) map(r).get("pos");
            check(!pos.startsWith(dir), "a refusal in a resolve-only file: " + pos + " " + map(r).get("reason"));
        }
    }

    /** Nothing of the document, no type, constant, function or statement, lies in {@code file}: it is refused, so it is not read. */
    private static void nothingReadFrom(Map<String, Object> doc, String file) {
        List<String> found = new ArrayList<>();
        for (String part : List.of("types", "consts", "funcs")) {
            positions(doc.get(part), found);
        }
        found.removeIf(p -> !p.startsWith(file + ":"));
        check(found.isEmpty(), file + " is refused as a whole and read at " + found);
    }

    private static void positions(Object o, List<String> out) {
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            if (m.get("pos") instanceof String) {
                out.add((String) m.get("pos"));
            }
            for (Object v : m.values()) {
                positions(v, out);
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                positions(v, out);
            }
        }
    }

    /**
     * Groovy's own logging transformation ({@code @Slf4j}, {@code @Commons}, {@code @Log4j}) loads
     * the logger class as it runs and throws {@code GroovyRuntimeException: Unable to load logging
     * class} when it is on no jar. The compiler turns that into an error that names no file. Each
     * service carrying such an annotation is refused at it, naming the class that is missing; the
     * resolve-only file is left out; the other service is read.
     */
    private void aLoggingAnnotationWithoutItsLoggerRefusesItsFile() throws Exception {
        for (String cls : List.of("org.slf4j.Logger", "org.apache.commons.logging.Log", "org.apache.log4j.Logger")) {
            notLoadable(cls);
        }
        Path app = appWithPlainService("logging-without-logger");
        String logged = "grails-app/services/lib/LoggedService.groovy";
        write(app.resolve(logged), "package lib\n\nimport groovy.util.logging.Slf4j\n\n@Slf4j\nclass LoggedService {\n\n"
            + "    int one() {\n        log.info(\"one\")\n        return 1\n    }\n}\n");
        String noted = "grails-app/services/lib/NotedService.groovy";
        write(app.resolve(noted), "package lib\n\nimport groovy.util.logging.Commons\n\n@Commons\nclass NotedService {\n\n"
            + "    int two() {\n        return 2\n    }\n}\n");
        write(app.resolve("src/groovy/lib/Notes.groovy"), "package lib\n\nimport groovy.util.logging.Log4j\n\n@Log4j\n"
            + "class Notes {\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of());
        refusedAsAWhole(doc, logged, 5, "unresolved: ", "LoggedService.one");
        String why = reasonsAt(doc, logged + ":5").get(0);
        check(why.contains("Unable to load logging class") && why.contains("org.slf4j.Logger"),
            "the reason does not hold the exception's message and the missing class org.slf4j.Logger: " + why);
        refusedAsAWhole(doc, noted, 5, "unresolved: ", "NotedService.two");
        check(reasonsAt(doc, noted + ":5").get(0).contains("org.apache.commons.logging.Log"),
            "the reason does not name the missing class org.apache.commons.logging.Log: " + reasonsAt(doc, noted + ":5"));
        nothingFrom(doc, "src/");
        check(list(doc.get("refusals")).size() == 2, "refusals " + Json.write(doc.get("refusals")));
    }

    /**
     * A trait whose file is left out for its import, while a jar holds an interface of its name:
     * the classes that call {@code Greets.super.hello()} then name a class that is no trait, and
     * the compiler reports "The usage of 'Class.this' and 'Class.super' is only allowed in
     * nested/inner classes." at that line, a message the reader has no rule for. The service is
     * refused there with the message; the resolve-only class is left out; the other service is read.
     */
    private void aTraitSuperCallWhoseTraitWasLeftOutRefusesItsFile() throws Exception {
        Path jar = jarOf("greets", Map.of("lib.Greets", "package lib;\n\npublic interface Greets {\n    String hello();\n}\n"),
            e -> true);
        Path app = appWithPlainService("trait-super-call");
        write(app.resolve("src/groovy/lib/Greets.groovy"), "package lib\n\nimport gone.Missing\n\ntrait Greets {\n\n"
            + "    String hello() {\n        return \"hello\"\n    }\n}\n");
        write(app.resolve("src/groovy/lib/Loud.groovy"), "package lib\n\nclass Loud implements Greets {\n\n"
            + "    String hello() {\n        return Greets.super.hello() + \"!\"\n    }\n}\n");
        String svc = "grails-app/services/lib/GreeterService.groovy";
        write(app.resolve(svc), "package lib\n\nclass GreeterService implements Greets {\n\n"
            + "    String hello() {\n        return Greets.super.hello()\n    }\n\n    int one() {\n        return 1\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        refusedAsAWhole(doc, svc, 6, "unsupported: compile error: The usage of 'Class.this' and 'Class.super' is only allowed in"
            + " nested/inner classes.", "GreeterService.one");
        nothingReadFrom(doc, svc);
        nothingFrom(doc, "src/");
        check(list(doc.get("refusals")).size() == 1, "refusals " + Json.write(doc.get("refusals")));
    }

    /**
     * A local AST transformation of a jar that throws as it runs, as a transformation of a
     * framework does once the framework is not running: {@code IllegalArgumentException}. The
     * reader does not run a local transformation of a jar. The annotation is a mark, refused at its
     * line as every annotation with behaviour is; the class that carries it is read as written, a
     * call of a member the transformation would have added stays a call the type checker does not
     * resolve, and a resolve-only class that carries it is compiled, so the service naming it
     * resolves.
     */
    private void aTransformationOfAJarIsNotRun() throws Exception {
        Map<String, String> sources = new LinkedHashMap<>();
        sources.put("xf.Checked", "package xf;\n\n@java.lang.annotation.Retention(java.lang.annotation.RetentionPolicy.RUNTIME)\n"
            + "@java.lang.annotation.Target(java.lang.annotation.ElementType.TYPE)\n"
            + "@org.codehaus.groovy.transform.GroovyASTTransformationClass(\"xf.CheckedTransform\")\n"
            + "public @interface Checked {\n}\n");
        sources.put("xf.CheckedTransform", "package xf;\n\n"
            + "@org.codehaus.groovy.transform.GroovyASTTransformation(phase = org.codehaus.groovy.control.CompilePhase.CANONICALIZATION)\n"
            + "public class CheckedTransform implements org.codehaus.groovy.transform.ASTTransformation {\n"
            + "    public void visit(org.codehaus.groovy.ast.ASTNode[] nodes, org.codehaus.groovy.control.SourceUnit source) {\n"
            + "        throw new IllegalArgumentException(\"Loader must not be null\");\n    }\n}\n");
        Path jar = jarOf("checked", sources, List.of(groovyJar()), e -> true, (entry, bytes) -> bytes);
        Path app = appWithPlainService("transformation-throws");
        write(app.resolve("src/groovy/lib/Form.groovy"), "package lib\n\nimport xf.Checked\n\n@Checked\nclass Form {\n\n"
            + "    int pages = 0\n}\n");
        String svc = "grails-app/services/lib/FormService.groovy";
        write(app.resolve(svc), "package lib\n\nimport xf.Checked\n\n@Checked\nclass FormService {\n\n"
            + "    boolean checked() {\n        return check()\n    }\n\n    int one() {\n        return 1\n    }\n}\n");
        String desk = "grails-app/services/lib/DeskService.groovy";
        write(app.resolve(desk), "package lib\n\nclass DeskService {\n\n"
            + "    int pagesOf(Form form) {\n        return 1\n    }\n\n    int two() {\n        return 2\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        for (Object r : list(doc.get("refusals"))) {
            check(!((String) map(r).get("reason")).startsWith("unresolved"), map(r).get("pos") + " " + map(r).get("reason"));
        }
        check(reasonsAt(doc, svc + ":5").equals(List.of("unsupported: annotation xf.Checked: annotations with behaviour are outside cir/1")),
            "@Checked at " + svc + ":5 is " + reasonsAt(doc, svc + ":5"));
        check(funcByKeyOrNull(doc, "FormService.one") != null, "FormService.one, of the class that carries the annotation, is not read: "
            + Json.write(doc.get("refusals")));
        foreignWith(doc, svc + ":9", "dynamic: ");
        check(reasonsAt(doc, desk + ":5").equals(List.of("type: type lib.Form is outside the reader's types")),
            "pagesOf at " + desk + ":5 is " + reasonsAt(doc, desk + ":5"));
        check(funcByKeyOrNull(doc, "DeskService.two") != null, "DeskService.two is not read");
        nothingFrom(doc, "src/");
    }

    /**
     * Two compiler errors the reader has no rule for, each with its file and line: a syntax error,
     * met while the file is parsed, and a local declared twice, met in a later phase. Each service
     * is refused at its line with the compiler's message; a resolve-only file that does not parse
     * is left out; the other service is read.
     */
    private void aCompileErrorNeverSeenRefusesItsFile() throws Exception {
        Path app = appWithPlainService("compile-errors");
        String broken = "grails-app/services/lib/BrokenService.groovy";
        write(app.resolve(broken), "package lib\n\nclass BrokenService {\n\n    int one( {\n        return 1\n    }\n}\n");
        String twice = "grails-app/services/lib/TwiceService.groovy";
        write(app.resolve(twice), "package lib\n\nclass TwiceService {\n\n"
            + "    int twice(int n) {\n        int n = 2\n        return n\n    }\n}\n");
        write(app.resolve("src/groovy/lib/Scrap.groovy"), "package lib\n\nclass Scrap {\n    int ( }\n");
        Map<String, Object> doc = readWritesADocument(app, List.of());
        refusedAsAWhole(doc, broken, 5, "unsupported: compile error: unexpected token: {", "BrokenService.one");
        refusedAsAWhole(doc, twice, 6, "unsupported: compile error: The current scope already contains a variable of the name n",
            "TwiceService.twice");
        nothingReadFrom(doc, broken);
        nothingReadFrom(doc, twice);
        nothingFrom(doc, "src/");
        check(list(doc.get("refusals")).size() == 2, "refusals " + Json.write(doc.get("refusals")));
    }

    /**
     * Code of a jar that the compiler runs inside a phase and that throws what nobody listed: the
     * processor of an annotation collector, {@code IllegalStateException}. The compiler knows
     * which file it was working on; that service is refused at its class with the exception's class
     * and message, a resolve-only file is left out, and the other service is read.
     */
    private void anExceptionInsideAPhaseRefusesTheFileBeingCompiled() throws Exception {
        Map<String, String> sources = new LinkedHashMap<>();
        sources.put("xf.Boom", "package xf;\n\n@java.lang.annotation.Retention(java.lang.annotation.RetentionPolicy.RUNTIME)\n"
            + "@java.lang.annotation.Target(java.lang.annotation.ElementType.TYPE)\n"
            + "@groovy.transform.AnnotationCollector(processor = \"xf.BoomProcessor\")\n"
            + "public @interface Boom {\n}\n");
        sources.put("xf.BoomProcessor", "package xf;\n\n"
            + "public class BoomProcessor extends org.codehaus.groovy.transform.AnnotationCollectorTransform {\n"
            + "    @Override\n"
            + "    public java.util.List<org.codehaus.groovy.ast.AnnotationNode> visit(org.codehaus.groovy.ast.AnnotationNode collector,\n"
            + "        org.codehaus.groovy.ast.AnnotationNode usage, org.codehaus.groovy.ast.AnnotatedNode annotated,\n"
            + "        org.codehaus.groovy.control.SourceUnit source) {\n"
            + "        throw new IllegalStateException(\"made up failure 4711\");\n    }\n}\n");
        Path jar = jarOf("boom", sources, List.of(groovyJar()), e -> true, (entry, bytes) -> bytes);
        Path app = appWithPlainService("exception-in-phase");
        String svc = "grails-app/services/lib/BoomService.groovy";
        write(app.resolve(svc), "package lib\n\nimport xf.Boom\n\n@Boom\nclass BoomService {\n\n"
            + "    int one() {\n        return 1\n    }\n}\n");
        write(app.resolve("src/groovy/lib/Bang.groovy"), "package lib\n\nimport xf.Boom\n\n@Boom\nclass Bang {\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        refusedAsAWhole(doc, svc, 5, "unsupported: ", "BoomService.one");
        String why = reasonsAt(doc, svc + ":5").get(0);
        check(why.contains("java.lang.IllegalStateException: made up failure 4711"),
            "the reason does not hold the exception's class and message: " + why);
        nothingReadFrom(doc, svc);
        nothingFrom(doc, "src/");
        check(list(doc.get("refusals")).size() == 1, "refusals " + Json.write(doc.get("refusals")));
    }

    /**
     * An exception that leaves the compiler outside the work on any one file: {@code @ASTTest},
     * Groovy's own, runs its closure from the compiler's progress callback, where whatever it
     * throws passes through the compilation. No file can be blamed, so every artefact is refused
     * at its first line with that one reason, and the document is written.
     */
    private void anExceptionThatNamesNoFileRefusesEveryArtefact() throws Exception {
        Path app = appWithPlainService("exception-names-no-file");
        String svc = "grails-app/services/lib/TestedService.groovy";
        write(app.resolve(svc), "package lib\n\nimport groovy.transform.ASTTest\n\n"
            + "@ASTTest(value = { throw new IllegalStateException(\"made up failure 4712\") })\nclass TestedService {\n\n"
            + "    int one() {\n        return 1\n    }\n}\n");
        Map<String, Object> doc;
        try {
            doc = Main.read(app, List.of());
        } catch (Throwable e) {
            throw new AssertionError("the read ended without a document: " + e);
        }
        String prefix = "unsupported: no file can be blamed: ";
        refusedAsAWhole(doc, svc, 1, prefix, "TestedService.one");
        refusedAsAWhole(doc, PLAIN_SERVICE, 1, prefix, "PlainService.pure");
        check(reasonsAt(doc, svc + ":1").equals(reasonsAt(doc, PLAIN_SERVICE + ":1")), "the two artefacts are refused for two reasons: "
            + Json.write(doc.get("refusals")));
        check(list(doc.get("refusals")).size() == 2, "refusals " + Json.write(doc.get("refusals")));
        check(list(doc.get("types")).isEmpty() && list(doc.get("funcs")).isEmpty(), "something is read beside the refusals");
        check("library".equals(doc.get("name")), "module name " + doc.get("name"));
    }

    /**
     * Three read files, each of which compiles only while the next is there. The trait's import
     * does not resolve, so it is refused; the service that calls {@code Gamma.super.name()} then
     * names the jar's interface of that name, which is no trait, and is refused with the compiler's
     * message; the service that names that one in a static method is refused in turn. The read
     * ends: each of the three is refused once, none of them is read, the fourth service is.
     */
    private void aChainOfThreeFilesEndsWithEachRefusedOnce() throws Exception {
        Path jar = jarOf("gamma", Map.of("lib.Gamma", "package lib;\n\npublic interface Gamma {\n    String name();\n}\n"), e -> true);
        Path app = appWithPlainService("chain-of-three");
        String gamma = "grails-app/domain/lib/Gamma.groovy";
        write(app.resolve(gamma), "package lib\n\nimport gone.Missing\n\ntrait Gamma {\n\n"
            + "    String name() {\n        return \"gamma\"\n    }\n}\n");
        String beta = "grails-app/services/lib/BetaService.groovy";
        write(app.resolve(beta), "package lib\n\nclass BetaService implements Gamma {\n\n    static final int TWO = 2\n\n"
            + "    String name() {\n        return Gamma.super.name()\n    }\n}\n");
        String alpha = "grails-app/services/lib/AlphaService.groovy";
        write(app.resolve(alpha), "package lib\n\nclass AlphaService {\n\n"
            + "    static int one() {\n        return BetaService.TWO - 1\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        refusedAsAWhole(doc, gamma, 3, "unresolved: gone.Missing", "Gamma.name");
        refusedAsAWhole(doc, beta, 8, "unsupported: compile error: The usage of 'Class.this' and 'Class.super'", "BetaService.name");
        refusedAsAWhole(doc, alpha, 6, "unresolved: BetaService", "one");
        for (String file : List.of(gamma, beta, alpha)) {
            nothingReadFrom(doc, file);
        }
        check(list(doc.get("refusals")).size() == 3, "refusals " + Json.write(doc.get("refusals")));
    }

    /**
     * A library class compiled for Java 8 whose member is a {@code javax.activation} or a JAXB
     * class: the JDK shipped both until Java 10 and no longer does, so on this runtime the class
     * is found and cannot be defined. The service that imports it is refused at its import,
     * naming the missing class; the other service is read.
     */
    private void aClassThatNeedsARemovedJdkPackageRefusesTheFileNamingIt() throws Exception {
        notLoadable("javax.activation.DataHandler");
        notLoadable("javax.xml.bind.JAXBContext");
        Map<String, String> sources = new LinkedHashMap<>();
        sources.put("legacy.Mailer", "package legacy;\n\npublic class Mailer {\n    public static javax.activation.DataHandler handler;\n\n"
            + "    public static int one() {\n        return 1;\n    }\n}\n");
        sources.put("legacy.Binder", "package legacy;\n\npublic class Binder {\n    public static javax.xml.bind.JAXBContext context;\n\n"
            + "    public static int two() {\n        return 2;\n    }\n}\n");
        Path jar = jarOf("legacy", sources, e -> true);
        Path app = appWithPlainService("removed-jdk-package");
        String mail = "grails-app/services/lib/MailService.groovy";
        write(app.resolve(mail), "package lib\n\nimport legacy.Mailer\n\nclass MailService {\n\n"
            + "    int one() {\n        return Mailer.one()\n    }\n}\n");
        String bind = "grails-app/services/lib/BindService.groovy";
        write(app.resolve(bind), "package lib\n\nimport legacy.Binder\n\nclass BindService {\n\n"
            + "    int two() {\n        return Binder.two()\n    }\n}\n");
        Map<String, Object> doc = readWritesADocument(app, List.of(jar));
        refusedAsAWhole(doc, mail, 3, "unresolved: legacy.Mailer", "MailService.one");
        check(reasonsAt(doc, mail + ":3").get(0).contains("javax/activation/DataHandler"),
            "the reason does not name the missing class: " + reasonsAt(doc, mail + ":3"));
        refusedAsAWhole(doc, bind, 3, "unresolved: legacy.Binder", "BindService.two");
        check(reasonsAt(doc, bind + ":3").get(0).contains("javax/xml/bind/JAXBContext"),
            "the reason does not name the missing class: " + reasonsAt(doc, bind + ":3"));
    }

    /**
     * A service saved as ISO-8859-1: Groovy's compiler reads it, each byte that is no UTF-8 as a
     * replacement character, so what it compiled is not what the file says. The service is refused
     * as a whole at its first line; the other service is read.
     */
    private void aServiceThatIsNotUtf8IsRefused() throws Exception {
        Path app = appWithPlainService("not-utf-8");
        String svc = "grails-app/services/lib/LatinService.groovy";
        Files.createDirectories(app.resolve(svc).getParent());
        Files.write(app.resolve(svc), ("package lib\n\nclass LatinService {\n\n    String name() {\n        return \"gräße\"\n"
            + "    }\n}\n").getBytes(StandardCharsets.ISO_8859_1));
        Map<String, Object> doc = readWritesADocument(app, List.of());
        refusedAsAWhole(doc, svc, 1, "unsupported: the file is not UTF-8 text", "LatinService.name");
        nothingReadFrom(doc, svc);
        check(list(doc.get("refusals")).size() == 1, "refusals " + Json.write(doc.get("refusals")));
    }

    // ---- call lists and opaque functions: testdata/holds ----

    private static final String DESK = "grails-app/services/holds/DeskService.groovy";
    private static final String RECALL = "grails-app/services/holds/RecallService.groovy";
    private static final String EXTENDS_NOTICE = "unsupported: class extends holds.NoticeService: inheritance is outside cir/1";

    private Path holdsDir() {
        return testdata.resolve("holds");
    }

    private Map<String, Object> holds() throws Exception {
        return Main.read(holdsDir(), List.of());
    }

    /** {@code <file>:<line>} of the line of {@code file}, a path in the holds application, holding {@code text}. */
    private String holdsPos(String file, String text) throws IOException {
        return file + ":" + lineOf(holdsDir().resolve(file), text);
    }

    private static String ref(String k, String path, String recv, String name, String pos) {
        return "{\"k\":\"" + k + "\"," + (path.isEmpty() ? "" : "\"path\":\"" + path + "\",")
            + (recv.isEmpty() ? "" : "\"recv\":\"" + recv + "\",") + "\"name\":\"" + name + "\",\"pos\":\"" + pos + "\"}";
    }

    private static String method(String path, String recv, String name, String pos) {
        return ref("method", path, recv, name, pos);
    }

    private static String func(String path, String name, String pos) {
        return ref("func", path, "", name, pos);
    }

    private static String made(String path, String name, String pos) {
        return ref("new", path, "", name, pos);
    }

    private static String value(String sig, String pos) {
        return "{\"k\":\"value\",\"sig\":\"" + sig + "\",\"pos\":\"" + pos + "\"}";
    }

    private static String lambda(String calls, String pos) {
        return "{\"k\":\"lambda\",\"sig\":\"groovy.lang.Closure\",\"calls\":[" + calls + "],\"pos\":\"" + pos + "\"}";
    }

    private static Map<String, Object> opaqueOrNull(Map<String, Object> doc, String key) {
        for (Object o : list(doc.get("opaque"))) {
            Map<String, Object> m = map(o);
            String k = m.containsKey("recv") ? m.get("recv") + "." + m.get("name") : (String) m.get("name");
            if (key.equals(k)) {
                return m;
            }
        }
        return null;
    }

    /** The opaque function {@code key} at {@code pos} with {@code reason}, its call list {@code [calls]}. */
    private static void opaqueIs(Map<String, Object> doc, String key, String pos, String reason, String calls) {
        Map<String, Object> o = opaqueOrNull(doc, key);
        check(o != null, "no opaque function " + key + " in " + Json.write(doc.getOrDefault("opaque", List.of())));
        check(pos.equals(o.get("pos")), key + " is at " + o.get("pos") + ", not " + pos);
        check(reason.equals(o.get("reason")), key + " has the reason " + o.get("reason") + ", not " + reason);
        check(same(o.get("calls"), "[" + calls + "]"), key + " calls " + Json.write(o.get("calls")));
        check(funcByKeyOrNull(doc, key) == null, key + " is also a function");
    }

    private static void foreignCalls(Map<String, Object> doc, String pos, String calls) {
        for (Map<String, Object> f : foreigns(doc.get("funcs"))) {
            if (pos.equals(f.get("pos"))) {
                check(same(f.get("calls"), "[" + calls + "]"), "the foreign statement at " + pos + " calls " + Json.write(f.get("calls")));
                return;
            }
        }
        throw new AssertionError("no foreign statement at " + pos);
    }

    private void holdsForeignStatementsListTheirCalls() throws Exception {
        Map<String, Object> doc = holds();
        String shelve = holdsPos(DESK, "shelve(hold, today)");
        foreignCalls(doc, shelve, method("", "DeskService", "shelve", shelve));
        String each = holdsPos(DESK, "waiting.each");
        foreignCalls(doc, each, func("org.codehaus.groovy.runtime.DefaultGroovyMethods", "each", each) + ","
            + lambda(method("", "DeskService", "renew2", each), each));
    }

    private void holdsARefusedMethodIsOpaque() throws Exception {
        Map<String, Object> doc = holds();
        String call = holdsPos(DESK, "noticeService.pickupReady");
        opaqueIs(doc, "DeskService.shelve", holdsPos(DESK, "def shelve"),
            "dynamic: shelve returns def: its result has the type of whatever it returns",
            method("", "NoticeService", "pickupReady", call) + "," + method("", "Hold", "daysWaiting", call));
        check(reasonsAt(doc, holdsPos(DESK, "def shelve")).equals(
            List.of("dynamic: shelve returns def: its result has the type of whatever it returns")),
            "the refusal of shelve: " + reasonsAt(doc, holdsPos(DESK, "def shelve")));
    }

    private void holdsARefusedClassIsOpaque() throws Exception {
        Map<String, Object> doc = holds();
        check(byNameOrNull(list(doc.get("types")), "RecallService") == null, "RecallService, which extends a class, is a type");
        String grace = holdsPos(RECALL, "sent = graceDays()");
        opaqueIs(doc, "NewRecallService", holdsPos(RECALL, "RecallService() {"), EXTENDS_NOTICE,
            method("", "RecallService", "graceDays", grace));
        String renew = holdsPos(RECALL, "deskService.renew(hold)");
        String remind = holdsPos(RECALL, "remind(today");
        String slip = holdsPos(RECALL, "new Slip");
        String trim = holdsPos(RECALL, "slip.text().trim()");
        String archive = holdsPos(RECALL, "hold.archive(line)");
        opaqueIs(doc, "RecallService.recall", holdsPos(RECALL, "void recall"), EXTENDS_NOTICE,
            method("", "DeskService", "renew", renew) + "," + method("", "NoticeService", "remind", remind) + ","
            + func("holds.Slip", "<init>", slip) + "," + made("holds", "Slip", slip) + ","
            + method("java.lang", "String", "trim", trim) + "," + method("holds", "Slip", "text", trim) + ","
            + value("groovy dynamic call of archive", archive));
        opaqueIs(doc, "RecallService.graceDays", holdsPos(RECALL, "static int graceDays"), EXTENDS_NOTICE, "");
        check(list(doc.get("opaque")).size() == 5, "opaque functions: " + Json.write(doc.get("opaque")));
    }

    private void holdsOverloadsAreTwoFunctions() throws Exception {
        Map<String, Object> doc = holds();
        Map<String, Object> first = funcByKey(doc, "DeskService.renew");
        check(holdsPos(DESK, "void renew(Hold hold) {").equals(first.get("pos")), "DeskService.renew is at " + first.get("pos"));
        String remind = holdsPos(DESK, "noticeService.remind(days)");
        opaqueIs(doc, "DeskService.renew2", holdsPos(DESK, "void renew(Hold hold, int days)"),
            "unsupported: a second method named renew: overloading is outside cir/1", method("", "NoticeService", "remind", remind));
        // The call with one argument names the first, the call with two the second.
        check(same(list(opaqueOrNull(doc, "RecallService.recall").get("calls")).get(0),
            method("", "DeskService", "renew", holdsPos(RECALL, "deskService.renew(hold)"))), "recall does not call DeskService.renew");
        String each = holdsPos(DESK, "waiting.each");
        Map<String, Object> closure = map(list(foreigns(funcByKey(doc, "DeskService.renewAll")).get(0).get("calls")).get(1));
        check(same(closure.get("calls"), "[" + method("", "DeskService", "renew2", each) + "]"),
            "the closure of renewAll calls " + Json.write(closure.get("calls")));
        String readme = Files.readString(root.resolve("adapters/groovy/README.md"));
        check(readme.contains("`renew2`"), "README.md does not say how the second of two overloads is named");
    }

    /** {@code codespec <args>}: its exit status and what it printed on stdout. */
    private Object[] codespecPrints(String name, String... args) throws Exception {
        Path out = work.resolve(name);
        List<String> command = new ArrayList<>(List.of(codespec.toString()));
        command.addAll(List.of(args));
        Process p = new ProcessBuilder(command).redirectOutput(out.toFile()).redirectError(ProcessBuilder.Redirect.INHERIT).start();
        return new Object[] {p.waitFor(), Files.readString(out)};
    }

    /**
     * {@code NoticeService.pickupReady} is called by {@code DeskService.shelve} alone, which returns
     * {@code def} and so is refused at its declaration; {@code checkIn}, the entry, calls {@code
     * shelve} in a statement kept foreign. Without call lists the path ends at that statement.
     */
    private void holdsReachFromCheckIn() throws Exception {
        Path doc = work.resolve("holds.reach.cir.json");
        Files.writeString(doc, Json.write(holds()), StandardCharsets.UTF_8);
        Object[] ran = codespecPrints("holds.reach.json", "reach", doc.toString(), "--entry", "holds.DeskService.checkIn");
        check(ran[0].equals(0), "codespec reach exit " + ran[0]);
        Map<String, Object> module = map(list(map(Json.parse((String) ran[1])).get("modules")).get(0));
        List<Object> reached = list(module.get("reached"));
        List<Object> unreached = list(module.get("unreached"));
        check(reached.contains("DeskService.checkIn"), "the entry is not reached: " + reached);
        check(reached.contains("NoticeService.pickupReady") && !unreached.contains("NoticeService.pickupReady"),
            "NoticeService.pickupReady is not reached from DeskService.checkIn: reached " + reached + ", unreached " + unreached);
        check(reached.contains("DeskService.shelve"), "DeskService.shelve, the refused method between them, is not reached: " + reached);
        check(unreached.contains("NoticeService.remind"), "NoticeService.remind, which checkIn does not reach, is not unreached: "
            + unreached);
    }

    private void holdsReadTwiceIsCanonical() throws Exception {
        String a = Json.write(holds());
        check(a.equals(Json.write(holds())), "two reads of holds differ");
        Path doc = work.resolve("holds.raw.json");
        Files.writeString(doc, a, StandardCharsets.UTF_8);
        Object[] valid = codespecPrints("holds.validate.txt", "validate", "--canonical", doc.toString());
        check(valid[0].equals(0), "codespec validate --canonical failed on the holds read");
        // A check reads the call lists of foreign statements and opaque functions; exit 2 is an
        // input it refuses.
        Object[] checked = codespecPrints("holds.check.txt", "check", "unbounded-growth", doc.toString());
        check(checked[0].equals(0), "codespec check unbounded-growth exit " + checked[0] + ": " + checked[1]);
    }

    private void holdsEqualsTheCheckedInDocument() throws Exception {
        Path checkedIn = testdata.resolve("holds.read.cir.json");
        check(Files.isRegularFile(checkedIn), "no " + checkedIn);
        check(Json.write(holds()).equals(Files.readString(checkedIn)), "the holds read differs from " + checkedIn
            + "; regenerate it with task examples");
        check(!list(map(Json.parse(Files.readString(checkedIn))).get("opaque")).isEmpty(), checkedIn + " holds no opaque function");
    }

    private void everyForeignStatementCarriesACallList() throws Exception {
        List<String> bad = new ArrayList<>();
        Map<String, Map<String, Object>> docs = new LinkedHashMap<>();
        try (Stream<Path> dirs = Files.list(testdata.resolve("refuse"))) {
            for (Path d : dirs.sorted().toList()) {
                docs.put("refuse/" + d.getFileName(), Reader.read(d));
            }
        }
        docs.put("lending", lending());
        docs.put("holds", holds());
        int seen = 0;
        for (Map.Entry<String, Map<String, Object>> e : docs.entrySet()) {
            for (Map<String, Object> f : foreigns(e.getValue().get("funcs"))) {
                seen++;
                if (!(f.get("calls") instanceof List)) {
                    bad.add(e.getKey() + " " + f.get("pos"));
                }
            }
        }
        check(seen > 20, "only " + seen + " foreign statements in the fixtures");
        check(bad.isEmpty(), "foreign statements without a call list: " + bad);
    }

    /**
     * One refused method making a call of each kind. In the module: a constructor is the function
     * {@code New<Class>} and a value made, a static method a function, an instance method a method.
     * Outside it, as the Java reader names an extern: a static method and a constructor ({@code
     * <init>}) are functions of the class's qualified name, an instance method a method of the
     * class in its package, a GDK method the static method that implements it. A call the checker
     * does not resolve, a call of a closure and a method pointer name no function.
     */
    private void callRefsNameTheirTargets() throws Exception {
        Path dir = work.resolve("call-refs");
        write(dir.resolve("Counter.groovy"), "package tally\n\nclass Counter {\n\n    int total = 0\n\n"
            + "    Counter(int start) {\n        total = start\n    }\n\n"
            + "    static int twice(int n) {\n        return n + n\n    }\n\n"
            + "    int bump() {\n        total = total + 1\n        return total\n    }\n\n"
            + "    def mixed(def other, Closure<Integer> step, List<String> names) {\n"
            + "        Counter c = new Counter(twice(2))\n"
            + "        c.bump()\n"
            + "        String s = String.valueOf(Math.max(1, 2)).trim()\n"
            + "        StringBuilder b = new StringBuilder()\n"
            + "        new StringBuilder()\n"
            + "        other.poke()\n"
            + "        step.call()\n"
            + "        names.each { String n -> b.append(n) }\n"
            + "        def p = c.&bump\n"
            + "        return s\n"
            + "    }\n}\n");
        Map<String, Object> doc = Reader.read(dir);
        String at = "Counter.groovy:";
        opaqueIs(doc, "Counter.mixed", at + 20, "dynamic: mixed returns def: its result has the type of whatever it returns",
            func("", "NewCounter", at + 21) + "," + made("", "Counter", at + 21) + "," + func("", "twice", at + 21) + ","
            + method("", "Counter", "bump", at + 22) + ","
            + method("java.lang", "String", "trim", at + 23) + "," + func("java.lang.String", "valueOf", at + 23) + ","
            + func("java.lang.Math", "max", at + 23) + ","
            + func("java.lang.StringBuilder", "<init>", at + 24) + "," + made("java.lang", "StringBuilder", at + 24) + ","
            + func("java.lang.StringBuilder", "<init>", at + 25) + ","
            + value("groovy dynamic call of poke", at + 26) + ","
            + value("groovy.lang.Closure", at + 27) + ","
            + func("org.codehaus.groovy.runtime.DefaultGroovyMethods", "each", at + 28) + ","
            + lambda(method("java.lang", "StringBuilder", "append", at + 28), at + 28) + ","
            + lambda(value("groovy dynamic call of bump", at + 29), at + 29));
        check(list(doc.get("opaque")).size() == 1, "opaque functions: " + Json.write(doc.get("opaque")));
    }

    /**
     * A class none of whose members is read because a field of it is refused, and a method whose
     * quoted name is no identifier: each method is an opaque function under the name the source
     * gives it, and strict reading takes the document.
     */
    private void fieldRefusedClassAndQuotedNameAreOpaque() throws Exception {
        Path dir = work.resolve("opaque-names");
        write(dir.resolve("Book.groovy"), "package shelf\n\nclass Book {\n\n    String title\n\n"
            + "    int width() {\n        return title.length()\n    }\n}\n");
        write(dir.resolve("Calc.groovy"), "package shelf\n\nclass Calc {\n\n"
            + "    long \"add two\"(long a, long b) {\n        return Math.max(a, b)\n    }\n\n"
            + "    long plus(long a, long b) {\n        return a + b\n    }\n}\n");
        Map<String, Object> doc = Reader.read(dir);
        Map<String, Object> width = opaqueOrNull(doc, "Book.width");
        check(width != null && ((String) width.get("reason")).startsWith("type: property title holds null until assigned"),
            "Book.width: " + Json.write(doc.getOrDefault("opaque", List.of())));
        check(same(width.get("calls"), "[" + method("java.lang", "String", "length", "Book.groovy:8") + "]"),
            "Book.width calls " + Json.write(width.get("calls")));
        Map<String, Object> quoted = opaqueOrNull(doc, "Calc.add two");
        check(quoted != null && same(quoted.get("calls"), "[" + func("java.lang.Math", "max", "Calc.groovy:6") + "]"),
            "Calc.\"add two\": " + Json.write(doc.getOrDefault("opaque", List.of())));
        check(funcByKeyOrNull(doc, "Calc.plus") != null, "Calc.plus, beside it, was not read");
        Path out = work.resolve("opaque-names.json");
        Files.writeString(out, Json.write(doc), StandardCharsets.UTF_8);
        Object[] valid = codespecPrints("opaque-names.validate.txt", "validate", "--canonical", out.toString());
        check(valid[0].equals(0), "codespec validate --canonical refused the read");
    }

    /**
     * Two classes of one simple name are both refused, and one module has one namespace: a method
     * of either has no name there, so neither is a function, and a call of one says so instead of
     * naming the other's.
     */
    private void duplicateSimpleNamesYieldNoOpaqueFunction() throws Exception {
        Path app = work.resolve("duplicate-opaque-app");
        write(app.resolve("application.properties"), "app.grails.version=2.5.6\napp.name=dup\n");
        write(app.resolve("grails-app/domain/a/Book.groovy"),
            "package a\n\nclass Book {\n    int pages = 0\n\n    int size() {\n        return pages\n    }\n}\n");
        write(app.resolve("grails-app/domain/b/Book.groovy"),
            "package b\n\nclass Book {\n    int copies = 0\n\n    int size() {\n        return copies\n    }\n}\n");
        String svc = "grails-app/services/a/ShelfService.groovy";
        write(app.resolve(svc), "package a\n\nclass ShelfService {\n    def count(Book book) {\n        return book.size()\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        check(opaqueOrNull(doc, "Book.size") == null, "a Book.size is an opaque function: " + Json.write(doc.get("opaque")));
        opaqueIs(doc, "ShelfService.count", svc + ":4", "dynamic: count returns def: its result has the type of whatever it returns",
            value("groovy call of a.Book.size, which the module holds no function for", svc + ":5"));
    }

    // ---- conventions: a target the type checker did not give ----

    private static final String REMINDER = "grails-app/services/holds/ReminderService.groovy";
    private static final String HOLD = "grails-app/domain/holds/Hold.groovy";
    private static final String GORM = "grails-gorm";
    private static final String SERVICE_FIELD = "grails-service-field";

    /** A method call ref whose target a convention gave: {@code by} between the name and the position. */
    private static String methodBy(String path, String recv, String name, String by, String pos) {
        return "{\"k\":\"method\"," + (path.isEmpty() ? "" : "\"path\":\"" + path + "\",") + "\"recv\":\"" + recv + "\",\"name\":\""
            + name + "\",\"by\":\"" + by + "\",\"pos\":\"" + pos + "\"}";
    }

    private static void onlyReasonStartsWith(Map<String, Object> doc, String pos, String start) {
        List<String> reasons = reasonsAt(doc, pos);
        check(reasons.size() == 1 && reasons.get(0).startsWith(start), "the refusal at " + pos + ": " + reasons);
    }

    /**
     * A GORM method called on a domain class, on an instance of one and on {@code this} in one is
     * a call of {@code grails.gorm.<Domain>.<method>}, marked {@code grails-gorm}: a dynamic
     * finder, {@code withNewTransaction} with the calls of its closure, {@code delete} and {@code
     * save}.
     */
    private void holdsGormCallsAreMarked() throws Exception {
        Map<String, Object> doc = holds();
        String count = holdsPos(REMINDER, "Hold.countByShelvedOnDay");
        foreignCalls(doc, count, methodBy("grails.gorm", "Hold", "countByShelvedOnDay", GORM, count));
        onlyReasonStartsWith(doc, count, "gorm: countByShelvedOnDay on Hold");
        String tx = holdsPos(REMINDER, "Hold.withNewTransaction");
        String each = holdsPos(REMINDER, "done.each");
        foreignCalls(doc, tx, methodBy("grails.gorm", "Hold", "withNewTransaction", GORM, tx) + ","
            + lambda(func("org.codehaus.groovy.runtime.DefaultGroovyMethods", "each", each) + ","
                + lambda(methodBy("grails.gorm", "Hold", "delete", GORM, each), each), tx));
        onlyReasonStartsWith(doc, tx, "gorm: withNewTransaction on Hold");
        String save = holdsPos(HOLD, "save()");
        foreignCalls(doc, save, methodBy("grails.gorm", "Hold", "save", GORM, save));
        onlyReasonStartsWith(doc, save, "gorm: save on Hold");
    }

    /**
     * {@code noticeService} and {@code deskService} are {@code def} fields of a service: the type
     * checker resolves no call through them. Grails 2 sets each by its name to the service of that
     * name, so the call is one of that service's method, marked {@code grails-service-field}; of
     * the two overloads of {@code renew} the one taking two arguments. A call through the typed
     * field of {@code DeskService} is resolved by the checker and carries no mark.
     */
    private void holdsServiceFieldCallsAreMarked() throws Exception {
        Map<String, Object> doc = holds();
        String remind = holdsPos(REMINDER, "noticeService.remind(late)");
        foreignCalls(doc, remind, methodBy("", "NoticeService", "remind", SERVICE_FIELD, remind));
        onlyReasonStartsWith(doc, remind, "spring: call through an injected dependency, noticeService.remind");
        String renew = holdsPos(REMINDER, "deskService.renew(hold, 7)");
        foreignCalls(doc, renew, methodBy("", "DeskService", "renew2", SERVICE_FIELD, renew));
        String ready = holdsPos(DESK, "noticeService.pickupReady");
        check(same(list(opaqueOrNull(doc, "DeskService.shelve").get("calls")).get(0), method("", "NoticeService", "pickupReady", ready)),
            "the call through the typed field: " + Json.write(opaqueOrNull(doc, "DeskService.shelve").get("calls")));
    }

    /** The first value of each row {@code codespec query} printed. */
    private static List<String> firstColumn(String printed) {
        List<String> out = new ArrayList<>();
        for (String line : printed.split("\n")) {
            if (!line.isEmpty()) {
                out.add(line.split("\t")[0]);
            }
        }
        return out;
    }

    /**
     * {@code ReminderService.remindLate} reaches {@code NoticeService.remind} and {@code
     * DeskService.renew2} only through {@code def} fields: {@code codespec reach} follows the
     * convention edges without being asked to. The relation {@code call_by} holds each, so a query
     * selects them, by convention, or leaves them out.
     */
    private void holdsReachAndQueryOverConventions() throws Exception {
        Path doc = work.resolve("holds.conventions.cir.json");
        Files.writeString(doc, Json.write(holds()), StandardCharsets.UTF_8);
        Object[] ran = codespecPrints("holds.conventions.reach.json", "reach", doc.toString(), "--entry",
            "holds.ReminderService.remindLate");
        check(ran[0].equals(0), "codespec reach exit " + ran[0]);
        List<Object> reached = list(map(list(map(Json.parse((String) ran[1])).get("modules")).get(0)).get("reached"));
        check(reached.contains("NoticeService.remind") && reached.contains("DeskService.renew2"),
            "reach does not follow the convention edges from ReminderService.remindLate: " + reached);
        Object[] conv = codespecPrints("holds.conventions.txt", "query",
            "conv(C, B) :- call{site: S, callee: C}, call_by{site: S, by: B}.\n?- conv(C, B).", doc.toString());
        check(conv[0].equals(1), "codespec query exit " + conv[0] + ", not 1 (rows)");
        check(List.of(((String) conv[1]).split("\n")).equals(List.of(
            "DeskService.renew2\t" + SERVICE_FIELD,
            "NoticeService.remind\t" + SERVICE_FIELD,
            "grails.gorm.Hold.countByShelvedOnDay\t" + GORM,
            "grails.gorm.Hold.delete\t" + GORM,
            "grails.gorm.Hold.save\t" + GORM,
            "grails.gorm.Hold.withNewTransaction\t" + GORM)), "the calls a convention gave a target: " + conv[1]);
        Object[] gorm = codespecPrints("holds.gorm.txt", "query",
            "gorm(C) :- call{site: S, callee: C}, call_by{site: S, by: \"" + GORM + "\"}.\n?- gorm(C).", doc.toString());
        check(firstColumn((String) gorm[1]).size() == 4, "the GORM calls: " + gorm[1]);
        Object[] typed = codespecPrints("holds.typed.txt", "query",
            "typed(C) :- call{site: S, callee: C, kind: \"method\", listed: true}, !call_by{site: S}.\n?- typed(C).", doc.toString());
        List<String> resolved = firstColumn((String) typed[1]);
        check(resolved.contains("NoticeService.pickupReady") && resolved.contains("java.lang.String.trim"),
            "the method calls the type checker resolved: " + resolved);
        for (String callee : resolved) {
            check(!callee.startsWith("grails.gorm."), "a GORM call among the calls the type checker resolved: " + resolved);
        }
        // Reachability over the edges the type checker gave alone: remindLate reaches no service.
        Object[] strict = codespecPrints("holds.strict.txt", "query",
            "edge(F, T) :- call{site: S, func: F}, calls{site: S, target: T}, !call_by{site: S}.\n"
            + "strict(F, T) :- edge(F, T).\nstrict(F, T) :- strict(F, M), edge(M, T).\n"
            + "?- strict(\"holds.ReminderService.remindLate\", T).", doc.toString());
        check(List.of(((String) strict[1]).split("\n")).equals(List.of("holds.Hold.daysWaiting")),
            "what remindLate reaches without the convention edges: " + strict[1]);
    }

    /**
     * A convention gives a target only where it names one: not for a method the service does not
     * declare, not where two overloads take as many arguments as the call passes, not for a field
     * named for a service the module does not read, and not for a GORM name the domain class
     * declares itself. Outside a Grails application nothing sets a field by its name and no class
     * is a domain class: no call is marked.
     */
    private void conventionsDoNotGuess() throws Exception {
        Path app = work.resolve("convention-app");
        write(app.resolve("application.properties"), "app.grails.version=2.5.6\napp.name=conv\n");
        write(app.resolve("grails-app/domain/conv/Item.groovy"),
            "package conv\n\nclass Item {\n    int n = 0\n\n    void save() {\n        n = n + 1\n    }\n}\n");
        write(app.resolve("grails-app/services/conv/PickService.groovy"), "package conv\n\nclass PickService {\n"
            + "    int picked = 0\n\n"
            + "    void pick(int a) {\n        picked = picked + a\n    }\n\n"
            + "    void pick(String s) {\n        picked = picked + 1\n    }\n\n"
            + "    void drop(int a) {\n        picked = picked - a\n    }\n}\n");
        String svc = "grails-app/services/conv/MixService.groovy";
        write(app.resolve(svc), "package conv\n\nclass MixService {\n"
            + "    def pickService\n    def mailService\n    PickService typedService\n\n"
            + "    def mix(Item item, def x) {\n"
            + "        pickService.pick(x)\n"
            + "        pickService.missing()\n"
            + "        pickService.drop(1)\n"
            + "        mailService.send(x)\n"
            + "        typedService.drop(2)\n"
            + "        item.save()\n"
            + "        Item.get(1)\n"
            + "        return x\n"
            + "    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        opaqueIs(doc, "MixService.mix", svc + ":8", "dynamic: mix returns def: its result has the type of whatever it returns",
            value("groovy dynamic call of pick", svc + ":9") + ","
            + value("groovy dynamic call of missing", svc + ":10") + ","
            + methodBy("", "PickService", "drop", SERVICE_FIELD, svc + ":11") + ","
            + value("groovy dynamic call of send", svc + ":12") + ","
            + method("", "PickService", "drop", svc + ":13") + ","
            + method("", "Item", "save", svc + ":14") + ","
            + methodBy("grails.gorm", "Item", "get", GORM, svc + ":15"));
        Path out = work.resolve("convention-app.json");
        Files.writeString(out, Json.write(doc), StandardCharsets.UTF_8);
        Object[] valid = codespecPrints("convention-app.validate.txt", "validate", "--canonical", out.toString());
        check(valid[0].equals(0), "codespec validate --canonical refused the read");

        Path plain = work.resolve("convention-plain");
        write(plain.resolve("Item.groovy"), "package plain\n\nclass Item {\n    int n = 0\n}\n");
        write(plain.resolve("ItemService.groovy"),
            "package plain\n\nclass ItemService {\n    int size(Item item) {\n        return item.n\n    }\n}\n");
        write(plain.resolve("Shelf.groovy"), "package plain\n\nclass Shelf {\n    def itemService\n\n"
            + "    def mix(Item item) {\n        itemService.size(item)\n        Item.get(1)\n        return item\n    }\n}\n");
        Map<String, Object> read = Reader.read(plain);
        check(same(opaqueOrNull(read, "Shelf.mix").get("calls"), "[" + value("groovy dynamic call of size", "Shelf.groovy:7") + ","
            + value("groovy dynamic call of get", "Shelf.groovy:8") + "]"),
            "Shelf.mix, outside a Grails application, calls " + Json.write(opaqueOrNull(read, "Shelf.mix").get("calls")));
    }

    /** The classes whose public methods GORM adds to every domain class, static, instance and validation. */
    private static final List<String> GORM_API = List.of(
        "org.grails.datastore.gorm.GormStaticApi", "org.grails.datastore.gorm.GormInstanceApi",
        "org.grails.datastore.gorm.GormValidationApi");

    /** The methods of those classes that are the API object's own: its hook, its state and its settings. */
    private static final List<String> GORM_API_OWN = List.of(
        "methodMissing", "getGormDynamicFinders", "getGormPersistentEntity", "getValidationErrorsMap", "getValidationSkipMap",
        "getFailOnError", "isFailOnError", "setFailOnError", "getValidationException", "setValidationException", "getValidator",
        "setValidator", "getBeforeValidateHelper", "setBeforeValidateHelper", "getMetaClass", "setMetaClass", "invokeMethod",
        "getProperty", "setProperty");

    /**
     * Every public instance method the GORM API classes of the Grails 2.5.6 distribution declare
     * (grails-datastore-gorm) is in the reader's list, {@code withTransaction} and {@code
     * withNewTransaction} among them, so the list is the distribution's and not a selection.
     */
    private void gormListIsTheDistributions() throws Exception {
        String home = System.getenv("GRAILS_HOME");
        if (home == null || home.isEmpty()) {
            throw new Skip("GRAILS_HOME is not set; set it to a Grails 2.5.6 distribution to run this case");
        }
        List<java.net.URL> jars = new ArrayList<>();
        for (String dir : List.of("lib", "dist")) {
            try (Stream<Path> files = Files.walk(Path.of(home, dir))) {
                for (Path p : files.sorted().toList()) {
                    String n = p.getFileName().toString();
                    if (n.endsWith(".jar") && !n.endsWith("-sources.jar") && !n.endsWith("-javadoc.jar")) {
                        jars.add(p.toUri().toURL());
                    }
                }
            }
        }
        List<String> missing = new ArrayList<>();
        List<String> seen = new ArrayList<>();
        try (java.net.URLClassLoader loader = new java.net.URLClassLoader(jars.toArray(new java.net.URL[0]),
            ClassLoader.getPlatformClassLoader())) {
            for (String api : GORM_API) {
                Class<?> c = Class.forName(api, false, loader);
                for (java.lang.reflect.Method m : c.getDeclaredMethods()) {
                    int mod = m.getModifiers();
                    String n = m.getName();
                    if (!java.lang.reflect.Modifier.isPublic(mod) || java.lang.reflect.Modifier.isStatic(mod) || m.isSynthetic()
                        || n.contains("$") || GORM_API_OWN.contains(n)) {
                        continue;
                    }
                    seen.add(n);
                    if (!Reader.gormMethod(n)) {
                        missing.add(c.getSimpleName() + "." + n);
                    }
                }
            }
        }
        check(seen.contains("withTransaction") && seen.contains("withNewTransaction") && seen.contains("save")
            && seen.contains("validate"), "the GORM API classes declare " + seen);
        check(missing.isEmpty(), "GORM methods of the distribution the reader's list does not hold: " + missing);
    }

    private void readmeNamesTheConventions() throws Exception {
        String readme = Files.readString(root.resolve("adapters/groovy/README.md"));
        for (String mark : List.of(GORM, SERVICE_FIELD)) {
            check(readme.contains("`" + mark + "`"), "README.md does not name the mark `" + mark + "`");
        }
        List<String> unnamed = new ArrayList<>();
        for (String name : Reader.GORM_METHODS) {
            if (!readme.contains("`" + name + "`")) {
                unnamed.add(name);
            }
        }
        for (String prefix : Reader.GORM_PREFIXES) {
            if (!readme.contains("`" + prefix + "*`")) {
                unnamed.add(prefix + "*");
            }
        }
        java.util.Collections.sort(unnamed);
        check(unnamed.isEmpty(), "README.md does not name the GORM methods " + unnamed);
        check(readme.contains("call_by"), "README.md does not name the relation call_by");
    }

    // ---- adversary: reads whose meaning differs from Groovy's ----

    /** {@code <file>:<line>} of every line marked {@code // diverges} in a fixture under testdata/adversary. */
    private List<String> divergeMarkers(String fixture) throws IOException {
        List<String> found = new ArrayList<>();
        try (Stream<Path> files = Files.list(testdata.resolve("adversary/" + fixture))) {
            for (Path f : files.sorted().toList()) {
                List<String> lines = Files.readAllLines(f);
                for (int i = 0; i < lines.size(); i++) {
                    if (lines.get(i).contains("// diverges")) {
                        found.add(f.getFileName() + ":" + (i + 1));
                    }
                }
            }
        }
        check(!found.isEmpty(), "no // diverges marker in " + fixture);
        return found;
    }

    /**
     * Passes when the reader refuses at a marked line, or when no statement read at a marked line
     * (outside foreign) is {@code bad}: the shape whose meaning differs from what Groovy does.
     */
    private void notReadAs(String fixture, java.util.function.Predicate<Map<String, Object>> bad) throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("adversary/" + fixture));
        List<String> marks = divergeMarkers(fixture);
        for (Object r : list(doc.get("refusals"))) {
            if (marks.contains(map(r).get("pos"))) {
                return;
            }
        }
        List<String> wrong = new ArrayList<>();
        for (Map<String, Object> s : statements(doc.get("funcs"))) {
            if (marks.contains(s.get("pos")) && !"foreign".equals(s.get("k")) && bad.test(s)) {
                wrong.add(s.get("pos") + " " + Json.write(s).replaceAll("\\s+", " "));
            }
        }
        check(wrong.isEmpty(), "read with a meaning Groovy does not give it, no refusal at " + marks + ": " + wrong);
    }

    /** Every object under {@code o} carrying a {@code pos}: functions and statements. */
    private static List<Map<String, Object>> statements(Object o) {
        List<Map<String, Object>> out = new ArrayList<>();
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            if (m.containsKey("pos") && m.containsKey("k")) {
                out.add(m);
            }
            for (Object v : m.values()) {
                out.addAll(statements(v));
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                out.addAll(statements(v));
            }
        }
        return out;
    }

    /** An assignment, ++ or -- whose target is the field {@code copies} itself. */
    private static boolean writesFieldCopies(Map<String, Object> s) {
        Object k = s.get("k");
        if (!"assign".equals(k) && !"incr".equals(k) && !"decr".equals(k)) {
            return false;
        }
        Object target = "assign".equals(k) ? list(s.get("lhs")).get(0) : s.get("x");
        Map<String, Object> t = map(target);
        return "field".equals(t.get("k")) && "copies".equals(t.get("name"));
    }

    // ---- helpers ----

    private static void check(boolean ok, String what) {
        if (!ok) {
            throw new AssertionError(what);
        }
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> map(Object o) {
        check(o instanceof Map, "not an object: " + o);
        return (Map<String, Object>) o;
    }

    @SuppressWarnings("unchecked")
    private static List<Object> list(Object o) {
        if (o == null) {
            return List.of();
        }
        check(o instanceof List, "not an array: " + o);
        return (List<Object>) o;
    }

    private static Map<String, Object> byNameOrNull(List<Object> xs, String name) {
        for (Object x : xs) {
            if (name.equals(map(x).get("name"))) {
                return map(x);
            }
        }
        return null;
    }

    private static Map<String, Object> byName(List<Object> xs, String name) {
        Map<String, Object> m = byNameOrNull(xs, name);
        check(m != null, "no " + name);
        return m;
    }

    private static Map<String, Object> funcByKeyOrNull(Map<String, Object> doc, String key) {
        for (Object f : list(doc.get("funcs"))) {
            Map<String, Object> fn = map(f);
            String k = (String) fn.get("name");
            if (fn.containsKey("recv")) {
                Map<String, Object> t = map(map(fn.get("recv")).get("t"));
                if ("ptr".equals(t.get("k"))) {
                    t = map(t.get("elem"));
                }
                k = t.get("name") + "." + k;
            }
            if (key.equals(k)) {
                return fn;
            }
        }
        return null;
    }

    private static Map<String, Object> funcByKey(Map<String, Object> doc, String key) {
        Map<String, Object> f = funcByKeyOrNull(doc, key);
        check(f != null, "no func " + key);
        return f;
    }

    private static Object returned(Map<String, Object> fn) {
        List<Object> body = list(fn.get("body"));
        Map<String, Object> ret = map(body.get(body.size() - 1));
        check("return".equals(ret.get("k")), "last statement is " + ret.get("k"));
        return list(ret.get("results")).get(0);
    }

    private static boolean same(Object got, String want) {
        return Json.write(got).equals(Json.write(Json.parse(want)));
    }

    /** Positions of every groovy foreign statement under {@code o}. */
    private static List<String> foreignPositions(Object o) {
        List<String> out = new ArrayList<>();
        for (Map<String, Object> f : foreigns(o)) {
            out.add((String) f.get("pos"));
        }
        return out;
    }

    private static List<String> foreignReasons(Object o) {
        List<String> out = new ArrayList<>();
        for (Map<String, Object> f : foreigns(o)) {
            out.add((String) f.get("reason"));
        }
        return out;
    }

    private static List<Map<String, Object>> foreigns(Object o) {
        List<Map<String, Object>> out = new ArrayList<>();
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            if ("foreign".equals(m.get("k")) && "groovy".equals(m.get("lang"))) {
                out.add(m);
            }
            for (Object v : m.values()) {
                out.addAll(foreigns(v));
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                out.addAll(foreigns(v));
            }
        }
        return out;
    }

    private static int lineOf(Path file, String text) throws IOException {
        List<String> lines = Files.readAllLines(file);
        for (int i = 0; i < lines.size(); i++) {
            if (lines.get(i).contains(text)) {
                return i + 1;
            }
        }
        throw new AssertionError("no " + text + " in " + file);
    }
}
