import factory.Envelope as Packet
fun message(): String = Packet<String>("ok").value
fun qualified(): String = factory.Envelope<String>("value").value
