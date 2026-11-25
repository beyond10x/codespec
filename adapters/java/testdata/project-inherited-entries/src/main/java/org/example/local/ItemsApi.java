package org.example.local;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;

@Path("/items")
public interface ItemsApi {
    @GET
    @Path("/{id}")
    String get(@PathParam("id") String id);

    @POST
    String create(String body);
}
