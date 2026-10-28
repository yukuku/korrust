package visibilityreflection
import visibilityprobe.*
import kotlin.reflect.KMutableProperty1
import kotlin.reflect.KVisibility
import kotlin.reflect.full.memberProperties
import kotlin.reflect.jvm.javaSetter
fun main() {
 val default = PrivateDefault::class.memberProperties.single { it.name == "value" } as KMutableProperty1<PrivateDefault, Int>
 check(default.visibility == KVisibility.PUBLIC)
 check(default.setter.visibility == KVisibility.PRIVATE)
 check(default.javaSetter == null)
 val custom = CustomPrivate::class.memberProperties.single { it.name == "value" } as KMutableProperty1<CustomPrivate, Int>
 check(custom.setter.visibility == KVisibility.PRIVATE)
 check(custom.javaSetter != null && java.lang.reflect.Modifier.isPrivate(custom.javaSetter!!.modifiers))
 val internal = InternalDefault::class.memberProperties.single { it.name == "value" } as KMutableProperty1<InternalDefault, Int>
 check(internal.visibility == KVisibility.INTERNAL)
 check(internal.setter.visibility == KVisibility.INTERNAL)
 check(internal.javaSetter!!.name == "setValue\$visibility_feature")
 val mixed = PublicInternalSetter::class.memberProperties.single { it.name == "value" } as KMutableProperty1<PublicInternalSetter, Int>
 check(mixed.visibility == KVisibility.PUBLIC)
 check(mixed.setter.visibility == KVisibility.INTERNAL)
 check(mixed.javaSetter!!.name == "setValue\$visibility_feature")
 println("OK")
}
