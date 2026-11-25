package org.example.api;

import jakarta.ws.rs.Consumes;
import jakarta.ws.rs.DELETE;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.PATCH;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.PUT;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;
import jakarta.ws.rs.Produces;

@Path("/items")
@Produces("application/json")
public interface ItemsApi {
    @GET
    @Path("/{id}")
    String get(@PathParam("id") String id);

    @POST
    @Consumes("text/plain")
    String create(String body);

    @PUT
    @Path("/{id}")
    String replace(@PathParam("id") String id, String body);

    @DELETE
    @Path("/{id}")
    String remove(@PathParam("id") String id);

    @PATCH
    @Path("/{id}")
    String patch(@PathParam("id") String id, String body);
}
