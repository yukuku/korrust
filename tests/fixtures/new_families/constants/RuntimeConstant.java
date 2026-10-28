package newfamilies.library;

@java.lang.annotation.Retention(java.lang.annotation.RetentionPolicy.RUNTIME)
@java.lang.annotation.Target(java.lang.annotation.ElementType.METHOD)
public @interface RuntimeConstant {
    int answer();
    String text();
    boolean flag();
}
