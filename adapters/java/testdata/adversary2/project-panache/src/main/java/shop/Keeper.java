package shop;

import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

@ApplicationScoped
public class Keeper {
    @Inject
    Svc.Helper helper;

    public boolean ready() {
        return helper != null;
    }
}
