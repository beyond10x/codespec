package org.example.clash;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;

@Path("/notes")
public interface ReadApi {
    @GET
    @Path("/{id}")
    String note(@PathParam("id") String id);
}
