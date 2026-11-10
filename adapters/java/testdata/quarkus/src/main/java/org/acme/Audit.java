package org.acme;

import org.jboss.logging.Logger;

public class Audit {
    private static final Logger LOGGER = Logger.getLogger(Audit.class.getName());

    public boolean verbose() {
        return LOGGER.isDebugEnabled();
    }
}
