package annotation17java;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
@Retention(RetentionPolicy.RUNTIME)
public @interface JavaNested {
    String text() default "java nested";
    long amount() default 37L;
}
