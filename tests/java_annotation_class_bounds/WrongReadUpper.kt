import annotationbounds.Bounds
import annotationbounds.Upper
import annotationbounds.Nested
fun bad(value: Bounds): kotlin.reflect.KClass<String> = value.error
