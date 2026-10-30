package annotation17java;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import java.lang.annotation.ElementType;
@Retention(RetentionPolicy.RUNTIME)
@Target({ElementType.TYPE, ElementType.METHOD})
public @interface JavaRich {
    String[] value() default {"java", "default"};
    long number() default Long.MIN_VALUE;
    float ratio() default -0.0f;
    double measure() default -0.0;
    char letter() default '\uD800';
    Class<?> type() default int[].class;
    Class<?>[] types() default {int.class, String[].class};
    java.lang.annotation.RetentionPolicy mode() default java.lang.annotation.RetentionPolicy.RUNTIME;
    JavaNested nested() default @JavaNested;
    JavaNested[] children() default {@JavaNested(text="child", amount=41L)};
}
