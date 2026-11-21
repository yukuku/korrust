package novjvm.consumer
import novjvm.*
import novjvm.aliases.qualified
import novjvm.aliases.qualifiedMonitor
fun main(args: Array<String>) {
    check(declared() == 7)
    check(qualified() == 24)
    check(qualifiedMonitor())
    check(named() == 8)
    check(spread() == 9)
    check(empty() == 10)
    check(defaulted() == 18)
    check(Defaults.value() == 19)
    check(Constructed().value == 22)
    check(Worker().enter())
    check(Worker().held)
    check(facadeMonitor())
    check(Singleton.instance())
    check(Singleton.staticMonitor())
    check(Host.both() == 1)
    check(Implementation().run() == 15)
    check(Implementation().ready() == 14)
    check(Child().inherited() == 12)
    check(Task().work() == 26)
    check(Wider().inherited() == 13)
    println("KOTLIN")
}
