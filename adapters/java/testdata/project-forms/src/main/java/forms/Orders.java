package forms;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.NotFoundException;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.WebApplicationException;

@Path("orders")
public class Orders {
    @GET
    @Path("{id}")
    public String get(Long id) {
        if (id < 0) {
            throw new WebApplicationException("Order with id of " + id + " does not exist.", 404);
        }
        return "order " + id;
    }

    @POST
    public String create(String name) {
        if (name.isEmpty()) {
            throw new NotFoundException("no name");
        }
        if (name.equals("none")) {
            throw new WebApplicationException("too long", 422);
        }
        return name;
    }

    @GET
    @Path("shown/{n}")
    public String show(Long n) {
        return "shown " + n; // refused: printed where Java prints a null one
    }

    public String label() {
        throw new WebApplicationException("no label", 500); // refused: no entry names
    }

    String twice(String name) {
        return create(name); // refused: which raises
    }

    String relay(Long n) {
        return show(n);
    }
}
