package org.example.loose;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;

public interface LooseApi {
    @GET
    @Path("/loose")
    String loose();
}
