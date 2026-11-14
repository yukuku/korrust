package use
import generics.*
fun consume(): String {
    val producer: Producer<Any> = Box<String>("OK")
    val consumer: Consumer<String> = Sink<Any>()
    val holder = Holder<Named>(Named("OK"))
    val upper: Bounded<Label> = holder
    return upper.get().text() + producer.get().toString() + consumer.accept("x")
}
