package directconsumer
import direct.*
fun compiled():String {
 val box:Covariant<Any> = Covariant<String>("OK")
 val sink:Contravariant<String> = Contravariant<Any>()
 return box.value.toString() + sink.accept("x")
}
