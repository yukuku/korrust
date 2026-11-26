package friends.consumer
import friends.library.*

private fun consume(counter: Counter): Int = counter.value()
fun total(): Int = Plain(3).bump() + Data().copy(value = 5).value +
    consume(Implementation(7)) + Singleton.value() + Choice.FIRST.value()
