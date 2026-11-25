package org.example.local;

public class ItemsResource implements ItemsApi {
    @Override
    public String get(String id) {
        return "item " + id;
    }

    @Override
    public String create(String body) {
        return body;
    }
}
