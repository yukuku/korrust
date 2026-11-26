package annotationbounds;
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME)
@Target({ElementType.TYPE, ElementType.METHOD})
public @interface Bounds {
    @Deprecated Class<? extends Throwable> error() default IllegalStateException.class;
    Class<Number> exact() default Number.class;
    Class<? super Number> lower() default Object.class;
    Class<? extends Cloneable> arrays() default int[].class;
    Class<? extends java.io.Serializable> serial() default String[].class;
    Class<Integer> integer() default Integer.class;
    Class<?> any() default String.class;
}
