package calleecases
import callee.Arena
import callee.Arena as Factory

fun constructorOnly() = Arena.CtorOnly()
fun methodOnly(): String = Arena.MethodOnly()
fun constructorSpecific(log: StringBuilder) = Arena.StringCtor(mark(log))
fun methodSpecific(log: StringBuilder): String = Arena.ObjectCtor(mark(log))
fun privateFactory(): String = Arena.PrivateCtor()
fun innerFactory(): String = Arena.InnerCtor()
fun hiddenFactory(): String = Arena.Hidden()
fun abstractFactory(): String = Arena.AbstractMethod()
fun aliasMethod(): String = Factory.MethodOnly()
fun fullyQualified(): String = callee.Arena.MethodOnly()
fun methodOnConstructorName(): String = Arena.CtorOnly(5)
fun constructorOnMethodName() = Factory.MethodOnly(5)
fun mark(log: StringBuilder): String {
    log.append("A")
    return "x"
}
