package friends.consumer
import friends.library.Counter
class PublicCounter : Counter { override fun value(): Int = 23 }
