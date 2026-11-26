package qualifiers;
import java.lang.annotation.*;
@javax.annotation.Nonnull
@javax.annotation.meta.TypeQualifierDefault({ElementType.METHOD, ElementType.PARAMETER, ElementType.FIELD})
@Retention(RetentionPolicy.CLASS)
@Target(ElementType.PACKAGE)
public @interface GlobalWarning {}
