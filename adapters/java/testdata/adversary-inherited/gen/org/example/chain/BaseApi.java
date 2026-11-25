package org.example.chain;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;

public interface BaseApi {
    @GET
    @Path("/ping")
    String ping();
}
