package direct
class Covariant<out T>(val value:T)
class Contravariant<in T> { fun accept(value:T):Int=17 }
fun source():String {
 val box:Covariant<Any> = Covariant<String>("OK")
 val sink:Contravariant<String> = Contravariant<Any>()
 return box.value.toString() + sink.accept("x")
}
