package consumer
import roots.makeVacancies
inline fun <reified T> relayVacancies(n: Int): Array<T?> = makeVacancies<T>(n)
