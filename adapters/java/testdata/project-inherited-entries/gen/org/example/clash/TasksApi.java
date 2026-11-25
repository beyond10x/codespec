package org.example.clash;

import jakarta.ws.rs.DELETE;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;

@Path("/tasks")
public interface TasksApi {
    @GET
    @Path("/{id}")
    String get(@PathParam("id") String id);

    @POST
    String create(String body);

    @DELETE
    @Path("/{id}")
    String remove(@PathParam("id") String id);
}
