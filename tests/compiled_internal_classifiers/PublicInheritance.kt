package friends.consumer
import friends.library.PublicImplementation
fun readPublic(value: PublicImplementation): Int = value.value() + value.base()
fun publicInheritance(): Int = readPublic(PublicImplementation())
