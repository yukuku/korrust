package marks;

import java.lang.annotation.*;

@Target(ElementType.PACKAGE)
@Retention(RetentionPolicy.CLASS)
public @interface Audit {}
