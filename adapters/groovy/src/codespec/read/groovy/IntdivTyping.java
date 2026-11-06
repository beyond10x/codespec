package codespec.read.groovy;

import java.util.List;
import java.util.Set;
import org.codehaus.groovy.ast.AnnotatedNode;
import org.codehaus.groovy.ast.AnnotationNode;
import org.codehaus.groovy.ast.ClassHelper;
import org.codehaus.groovy.ast.ClassNode;
import org.codehaus.groovy.ast.MethodNode;
import org.codehaus.groovy.ast.expr.Expression;
import org.codehaus.groovy.ast.expr.MethodCall;
import org.codehaus.groovy.ast.expr.MethodCallExpression;
import org.codehaus.groovy.ast.expr.TupleExpression;
import org.codehaus.groovy.transform.stc.ExtensionMethodNode;
import org.codehaus.groovy.transform.stc.StaticTypeCheckingVisitor;
import org.codehaus.groovy.transform.stc.TypeCheckingContext;
import org.codehaus.groovy.transform.stc.TypeCheckingExtension;

/**
 * Types {@code x.intdiv(y)} as Groovy runs it. The type checker types {@code
 * DefaultGroovyMethods.intdiv(Number, Number)} as {@code Number}, so {@code long q =
 * a.intdiv(b)} is a type-checking error, though at run time {@code NumberMath} gives an {@code
 * Integer} for two {@code int}s and a {@code Long} for two {@code long}s or an {@code int} and a
 * {@code long}. This extension stores that type, primitive, for those operands in code groovyc
 * compiles dynamically. It leaves every other {@code intdiv} as the checker types it, and every
 * one in {@code @CompileStatic} or {@code @TypeChecked} code, where groovyc itself refuses the
 * {@code Number}.
 */
final class IntdivTyping extends TypeCheckingExtension {
    private static final String DGM = "org.codehaus.groovy.runtime.DefaultGroovyMethods";

    IntdivTyping(StaticTypeCheckingVisitor v) {
        super(v);
    }

    @Override
    public void afterMethodCall(MethodCall call) {
        if (!(call instanceof MethodCallExpression)) {
            return;
        }
        if (staticallyChecked()) {
            return;
        }
        MethodCallExpression mce = (MethodCallExpression) call;
        if (!"intdiv".equals(mce.getMethodAsString()) || mce.isImplicitThis()
            || !(mce.getArguments() instanceof TupleExpression)) {
            return;
        }
        MethodNode target = getTargetMethod(mce);
        if (!(target instanceof ExtensionMethodNode)
            || !DGM.equals(((ExtensionMethodNode) target).getExtensionMethodNode().getDeclaringClass().getName())) {
            return;
        }
        List<Expression> args = ((TupleExpression) mce.getArguments()).getExpressions();
        if (args.size() != 1) {
            return;
        }
        ClassNode x = ClassHelper.getUnwrapper(getType(mce.getObjectExpression()));
        ClassNode y = ClassHelper.getUnwrapper(getType(args.get(0)));
        boolean xi = x.equals(ClassHelper.int_TYPE);
        boolean yi = y.equals(ClassHelper.int_TYPE);
        boolean xl = x.equals(ClassHelper.long_TYPE);
        boolean yl = y.equals(ClassHelper.long_TYPE);
        if (xi && yi) {
            storeType(mce, ClassHelper.int_TYPE);
        } else if ((xi || xl) && (yi || yl)) {
            storeType(mce, ClassHelper.long_TYPE);
        }
    }

    /**
     * Whether the code checked now is compiled statically: the innermost {@code @CompileStatic} or
     * {@code @TypeChecked} on the enclosing method, else its class or an outer class, is not {@code
     * SKIP}. There groovyc itself types {@code intdiv} as {@code Number} and refuses the program, so
     * the checker's error stands.
     */
    private boolean staticallyChecked() {
        TypeCheckingContext ctx = typeCheckingVisitor.getTypeCheckingContext();
        MethodNode m = ctx.getEnclosingMethod();
        if (m != null) {
            Boolean b = annotated(m);
            if (b != null) {
                return b;
            }
        }
        for (ClassNode c = ctx.getEnclosingClassNode(); c != null; c = c.getOuterClass()) {
            Boolean b = annotated(c);
            if (b != null) {
                return b;
            }
        }
        return false;
    }

    /** True for a static-checking annotation in a checking mode, false for one that says SKIP, null for none. */
    private static Boolean annotated(AnnotatedNode n) {
        for (AnnotationNode a : n.getAnnotations()) {
            String name = a.getClassNode().getName();
            if (STATIC.contains(name)) {
                Expression v = a.getMember("value");
                return v == null || !v.getText().endsWith("SKIP");
            }
        }
        return null;
    }

    private static final Set<String> STATIC = Set.of("groovy.transform.CompileStatic", "groovy.transform.TypeChecked");
}
