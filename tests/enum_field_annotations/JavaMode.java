package fieldmeta.dependency;
import java.lang.annotation.*;
public enum JavaMode {
    @org.jetbrains.annotations.Nullable OPTIONAL,
    @PathTag("type") TYPE_USE;
    @Retention(RetentionPolicy.RUNTIME)
    @Target(ElementType.TYPE_USE)
    public @interface PathTag { String value(); }
}
