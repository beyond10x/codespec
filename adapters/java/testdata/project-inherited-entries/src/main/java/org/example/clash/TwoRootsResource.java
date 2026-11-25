package org.example.clash;

public class TwoRootsResource implements RootApi, ExtraApi { // refused: interfaces with different @Path values
    @Override
    public String leaf() {
        return "leaf";
    }
}
