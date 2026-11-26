package markers
import kotlin.reflect.KClass
open class Base
class Child : Base()
@Retention(AnnotationRetention.RUNTIME)
annotation class Entries(vararg val classes: KClass<out Base>)
annotation class Flag
@Retention(AnnotationRetention.RUNTIME)
annotation class Flags(vararg val value: KClass<out Annotation>)
