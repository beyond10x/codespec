package shop;

import jakarta.enterprise.context.ApplicationScoped;

public interface Svc {
    long run();

    @ApplicationScoped
    class Helper {
        public long n;
    }
}
