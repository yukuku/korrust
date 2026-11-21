package novjvm.aliases
import kotlin.Throws as CommonThrows
import kotlin.jvm.Synchronized as Monitor
import java.io.IOException as Failure

typealias Declares = kotlin.jvm.Throws
typealias Locked = kotlin.jvm.Synchronized

@CommonThrows(Failure::class)
fun common(): Int = 20

@Declares(Failure::class)
fun sourceAlias(): Int = 21

@Locked
fun sourceMonitor(): Boolean = novjvm.support.Probe.staticHeld("novjvm.aliases.AliasesKt")

@kotlin.jvm.Throws(java.io.IOException::class)
fun qualified(): Int = 24

@kotlin.jvm.Synchronized
fun qualifiedMonitor(): Boolean = novjvm.support.Probe.staticHeld("novjvm.aliases.AliasesKt")
