package friends.consumer
import friends.library.Plain
class Exposed {
    fun exposed(value: Plain): Plain = value
}
