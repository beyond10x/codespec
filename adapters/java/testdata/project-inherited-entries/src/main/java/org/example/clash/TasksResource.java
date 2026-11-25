package org.example.clash;

import jakarta.ws.rs.DELETE;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;

public class TasksResource implements TasksApi {
    @POST
    @Override
    public String get(String id) { // refused: JAX-RS annotations of its own
        return "task";
    }

    @POST
    @Path("/new")
    @Override
    public String create(String body) { // refused: JAX-RS annotations of its own
        return body;
    }

    @DELETE
    @Path("/{id}")
    @Override
    public String remove(String id) {
        return "removed";
    }
}
