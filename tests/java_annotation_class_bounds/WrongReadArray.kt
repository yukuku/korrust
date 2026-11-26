import annotationbounds.Bounds
import annotationbounds.Upper
import annotationbounds.Nested
fun bad(value: Upper): Array<kotlin.reflect.KClass<String>> = value.value
