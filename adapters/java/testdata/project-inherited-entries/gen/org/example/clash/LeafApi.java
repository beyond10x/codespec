package org.example.clash;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;

public interface LeafApi {
    @GET
    @Path("/leaf")
    String leaf();
}
