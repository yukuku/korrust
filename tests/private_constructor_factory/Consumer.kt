import factory.Envelope as Packet
fun message(): String = Packet<String>("ok").value
