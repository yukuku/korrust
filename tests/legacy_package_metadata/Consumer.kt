package compatibility.consumer

import compatibility.library.Label
import compatibility.library.Marker
import compatibility.library.PACKAGE_CONSTANT
import compatibility.library.answer as importedAnswer
import compatibility.library.lengthPlus
import compatibility.library.namedAnswer
import compatibility.library.packageValue
import compatibility.library.relay
import compatibility.parts.*

@Marker("older")
class Tagged

fun value(): Int = importedAnswer() + relay(5) + "abc".lengthPlus(delta = 2) +
    packageValue + PACKAGE_CONSTANT + namedAnswer(value = 3) + first() + second()

fun label(value: Label): String = value
