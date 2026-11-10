package forms;

import forms.other.Kit;
import forms.other.Thing;

public class Uses {
    public long made() {
        return new Kit(2).size();
    }

    public Kit kit() {
        return Kit.of(3);
    }

    public Thing thing(Kit k) {
        return k.thing();
    }

    public Thing fresh() {
        return new Thing();
    }

    public String tag(Kit k) {
        return k.label("none");
    }
}
