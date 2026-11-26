package kotlin.annotations.jvm;
import java.lang.annotation.*;
@Retention(RetentionPolicy.CLASS)
@Target(ElementType.ANNOTATION_TYPE)
public @interface UnderMigration { MigrationStatus status(); }
