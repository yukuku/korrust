package compatibility.annotations
import java.lang.annotation.RetentionPolicy
@BoundaryMark(values = [1, 2], type = String::class, policy = RetentionPolicy.RUNTIME)
class AnnotationArguments
