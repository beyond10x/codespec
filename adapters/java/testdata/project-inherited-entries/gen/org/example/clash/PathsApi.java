package org.example.clash;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;

@Path("/paths")
public interface PathsApi {
    @GET
    String list();
}
