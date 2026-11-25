package org.example.clash;

import jakarta.ws.rs.Path;

@Path("/other")
public class PathsResource implements PathsApi { // refused: a @Path of its own
    @Override
    public String list() {
        return "paths";
    }
}
