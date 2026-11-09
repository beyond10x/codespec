package org.acme;

import jakarta.inject.Inject;
import jakarta.transaction.Transactional;
import jakarta.ws.rs.Consumes;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import org.eclipse.microprofile.rest.client.inject.RestClient;

@Path("/fruits")
@Produces("application/json")
public class FruitResource {
    @Inject
    FruitService service;

    @Inject
    @RestClient
    CountryClient countries;

    @GET
    @Path("/origin")
    @Produces({"text/plain", "text/html"})
    public String origin() {
        return countries.byName(service.defaultName());
    }

    @POST
    @Consumes("text/plain")
    @Transactional
    public String create(String name) {
        Fruit f = new Fruit();
        f.name = name;
        f.persist();
        return name;
    }
}
