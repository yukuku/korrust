package annotationboundscheck;

import annotationboundsuses.Robolectric;
import org.robolectric.annotation.Config;

public class RobolectricCheck {
    public static void main(String[] args) {
        Config config = Robolectric.class.getAnnotation(Config.class);
        if (config.application() != android.app.Application.class) throw new AssertionError("application");
        if (config.sdk().length != 1 || config.sdk()[0] != 34) throw new AssertionError("sdk");
        if (!config.manifest().equals(Config.NONE)) throw new AssertionError("deprecated element");
        if (config.shadows().length != 1 || config.shadows()[0] != String.class) throw new AssertionError("shadows");
        if (config.fontScale() != 1.0f || config.minSdk() != -1) throw new AssertionError("defaults");
        System.out.println("OK:robolectric");
    }
}
