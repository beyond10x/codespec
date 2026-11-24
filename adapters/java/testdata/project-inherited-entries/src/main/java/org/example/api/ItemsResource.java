package org.example.api;

import jakarta.enterprise.context.ApplicationScoped;

@ApplicationScoped
public class ItemsResource implements ItemsApi {
    @Override
    public String get(String id) {
        return "item " + id;
    }

    @Override
    public String create(String body) {
        return body;
    }

    @Override
    public String replace(String id, String body) {
        return "replaced " + id;
    }

    @Override
    public String remove(String id) {
        return "removed " + id;
    }

    @Override
    public String patch(String id, String body) {
        return "patched " + id;
    }
}
