package org.example.clash;

import jakarta.ws.rs.Path;

@Path("/own")
public class OwnRootResource implements RootApi { // refused: a @Path of its own
    @Override
    public String leaf() {
        return "leaf";
    }
}
