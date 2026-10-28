package visibilityconsumer
import visibilityprobe.*
fun main() {
 val privateDefault = PrivateDefault()
 privateDefault.assign(3)
 check(privateDefault.value == 3)
 privateDefault.deferred(7)()
 check(privateDefault.value == 7)
 val custom = CustomPrivate()
 custom.assign(9)
 check(custom.value == 10)
 val internal = InternalDefault()
 internal.value = 11
 check(internal.value == 11)
 val partial = PublicInternalSetter()
 partial.value = 12
 check(partial.value == 12)
 State.hidden = 13
 check(State.hidden == 13)
 Host.hidden = 14
 check(Host.hidden == 14)
 check(top == 1)
 Contract.hidden = 17
 check(Contract.hidden == 17)
 println("OK")
}
