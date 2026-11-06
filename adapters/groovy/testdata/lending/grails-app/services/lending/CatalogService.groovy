package lending

class CatalogService {

    TreeMap<String, String> shelfNotes = new TreeMap<>() // expect: read

    Tool toolById(long id) { // expect: refused
        return Tool.get(id) // expect: refused gorm
    }

    Tool toolNamed(String n) { // expect: refused
        return Tool.findByName(n) // expect: refused gorm
    }

    List<Tool> onShelf(String shelfName) { // expect: refused
        return Tool.withCriteria { // expect: refused gorm
            eq('shelf', shelfName)
        }
    }

    String shelfOf(Tool tool) { // expect: refused
        return tool?.shelf // expect: refused groovy-semantics
    }

    String nameOr(String name, String fallback) { // expect: refused
        return name ?: fallback // expect: refused groovy-semantics
    }

    List<String> names(List<Tool> tools) { // expect: refused
        return tools.collect { Tool t -> t.name } // expect: refused closure
    }

    int countLent(List<Tool> tools) { // expect: refused
        int lent = 0
        tools.each { Tool t -> // expect: refused closure
            lent = lent + t.timesLent
        }
        return lent
    }

    String describe(def item) { // expect: refused
        return item.label() // expect: refused dynamic
    }

    int shelfCount() { // expect: refused
        return countShelves() // expect: refused dynamic
    }

    void installShelfDefault() { // expect: refused
        Tool.metaClass.defaultShelf = "front" // expect: refused dynamic
    }

    int doubled(int n) { // expect: read
        def twice = n * 2
        return twice
    }

    int sumByShelf(TreeMap<String, Integer> perShelf) { // expect: refused
        int total = 0
        for (String shelf : perShelf.keySet()) { // expect: refused unsupported
            total = total + perShelf.get(shelf)
        }
        return total
    }

    String shelfList() { // expect: read
        String list = ""
        for (String shelf : shelfNotes.keySet()) { // expect: read
            list = list + shelf + ";"
        }
        return list
    }
}
