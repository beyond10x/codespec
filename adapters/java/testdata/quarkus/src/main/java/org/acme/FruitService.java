package org.acme;

import jakarta.enterprise.context.ApplicationScoped;

@ApplicationScoped
public class FruitService {
    public String defaultName() {
        return "apple";
    }
}
