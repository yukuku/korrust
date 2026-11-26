package annotationbounds;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
public @interface Nested { Upper value(); }
