package annotation17extra
import annotation17.*
@Target(AnnotationTarget.PROPERTY) annotation class PropertyMark(val value: Long = 1L)
class Good { @PropertyMark val value: Int = 1 }
