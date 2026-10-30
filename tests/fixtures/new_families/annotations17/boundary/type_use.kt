package annotation17cases
import annotation17.*
@Target(AnnotationTarget.TYPE) annotation class TypeMark
fun typed(value: @TypeMark String): String = value
