package constructors;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target({ElementType.CONSTRUCTOR, ElementType.PARAMETER})
public @interface Mark { String value(); }
