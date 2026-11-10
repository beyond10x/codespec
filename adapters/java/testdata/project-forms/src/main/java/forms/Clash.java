package forms;

import org.jboss.logging.Logger;

public class Clash {
    private static final Logger LOGGER = Logger.getLogger(Clash.class.getName());

    public boolean verbose() {
        return LOGGER.isDebugEnabled();
    }
}
