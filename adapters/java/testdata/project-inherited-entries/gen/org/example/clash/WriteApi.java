package org.example.clash;

import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;

@Path("/notes")
public interface WriteApi {
    @POST
    @Path("/{id}")
    String note(@PathParam("id") String id);
}
