package annotationbounds;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
public @interface Upper {
    Class<? extends Number>[] value() default { Integer.class };
}
