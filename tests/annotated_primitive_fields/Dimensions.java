package fixture;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
public final class Dimensions {
    @Retention(RetentionPolicy.RUNTIME)
    public @interface Item { int number(); String label(); }
    @Retention(RetentionPolicy.RUNTIME)
    public @interface Exported { Item[] mapping(); }
    @Exported(mapping = {@Item(number=-1,label="fill"), @Item(number=-2,label="wrap")})
    public int width = -1;
    @Exported(mapping = {}) public static long count = 10;
    @Exported(mapping = {}) public final int height = 11;
}
