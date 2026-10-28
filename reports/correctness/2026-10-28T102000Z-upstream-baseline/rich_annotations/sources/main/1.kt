
class O
class K

@JavaAnn(args = arrayOf(O::class, K::class)) class MyClass

fun box(): String {
    val args = MyClass::class.java.getAnnotation(JavaAnn::class.java).args
    val argName1 = args[0].java.simpleName ?: "fail 1"
    val argName2 = args[1].java.simpleName ?: "fail 2"
    return argName1 + argName2
}
