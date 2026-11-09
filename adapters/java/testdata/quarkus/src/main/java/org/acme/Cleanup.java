package org.acme;

import io.quarkus.scheduler.Scheduled;
import jakarta.enterprise.context.ApplicationScoped;

@ApplicationScoped
public class Cleanup {
    @Scheduled(every = "10m")
    public void sweep() {
        Fruit.deleteAll();
    }
}
