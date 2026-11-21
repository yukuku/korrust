package novjvm

import kotlin.jvm.Throws as Declares
import kotlin.jvm.Synchronized as Monitor
import java.io.IOException as IO
import novjvm.support.CustomFailure
import novjvm.support.Probe

typealias FailureAlias = CustomFailure

@Declares(IO::class, FailureAlias::class, Throwable::class)
fun declared(): Int = 7

@Throws(exceptionClasses = [IO::class, CustomFailure::class])
fun named(): Int = 8

@Throws(*[IO::class])
fun spread(): Int = 9

@Throws()
fun empty(): Int = 10

@Monitor
fun facadeMonitor(): Boolean = Probe.staticHeld("novjvm.FunctionsKt")

@Monitor @Declares(IO::class)
fun facadeFailure() { throw IO() }

class Worker @Declares(CustomFailure::class) constructor() {
    @Monitor
    fun enter(): Boolean = Thread.holdsLock(this)

    @Monitor @Declares(IO::class)
    fun fail() { throw IO() }

    @get:Monitor @get:Declares(IO::class)
    val held: Boolean get() = Thread.holdsLock(this)
}

open class Parent {
    @Declares(IO::class)
    open fun inherited(): Int = 11
}
class Child: Parent() {
    override fun inherited(): Int = 12
}
class Wider: Parent() {
    @Declares(Exception::class)
    override fun inherited(): Int = 13
}

interface Contract {
    @Declares(Throwable::class)
    fun run(): Int

    @Declares(CustomFailure::class)
    fun ready(): Int = 14
}
class Implementation: Contract {
    @Declares(IO::class)
    override fun run(): Int = 15

    @Declares(CustomFailure::class)
    override fun ready(): Int = 14
}

class Host {
    interface ThrowEverythingRunnable {
        @Throws(Throwable::class)
        fun run()
    }
    companion object {
        @JvmStatic @Monitor @Declares(IO::class)
        fun both(): Int = (if (Thread.holdsLock(this)) 1 else 0) + (if (Probe.staticHeld("novjvm.Host")) 2 else 0)
    }
}

object Singleton {
    @Monitor
    fun instance(): Boolean = Thread.holdsLock(this)

    @JvmStatic @Monitor @Declares(CustomFailure::class)
    fun staticMonitor(): Boolean = Probe.staticHeld("novjvm.Singleton")
}

@JvmOverloads @Declares(IO::class)
fun defaulted(value: Int = 18): Int = value

object Defaults {
    @JvmStatic @JvmOverloads @Throws(IO::class)
    fun value(value: Int = 19): Int = value
}

class Constructed @JvmOverloads @Declares(IO::class) constructor(val value: Int = 22)

class SourceFailure: Exception()
@Declares(SourceFailure::class)
fun sourceFailure(): Int = 23

abstract class AbstractTask {
    @Declares(IO::class)
    abstract fun work(): Int
}
class Task: AbstractTask() {
    @Declares(CustomFailure::class)
    override fun work(): Int = 26
}
