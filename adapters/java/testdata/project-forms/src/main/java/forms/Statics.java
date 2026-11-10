package forms;

import java.util.ArrayList;
import java.util.List;
import org.jboss.logging.Logger;

public class Statics {
    private static final List<String> NAMES = new ArrayList<>(); // refused: a static field initialized other than
    private static final String HOME = home(); // refused: a static field initialized other than
    private static final Logger BY_ENV = Logger.getLogger(System.getenv("LOG")); // refused: a static field initialized other than
    private static final Logger LOGGER = Logger.getLogger("statics"); // refused: a second module-level name LOGGER

    static String home() {
        return "home";
    }
}
