@file:Facade(name = "StableApi")
package nextjvm.api

import kotlin.jvm.JvmName as Facade

const val BASE: Int = 40
private typealias NumberAlias = Int
fun answer(): NumberAlias = BASE + 2
