package app.plantswap.frontend

import kotlinx.datetime.LocalDateTime
import kotlinx.serialization.KSerializer
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

@OptIn(ExperimentalUuidApi::class)
private data object UuidSerializer : KSerializer<Uuid> {
    override val descriptor: SerialDescriptor = String.serializer().descriptor

    override fun serialize(encoder: Encoder, value: Uuid) =
        encoder.encodeString(value.toString())

    override fun deserialize(decoder: Decoder): Uuid =
        Uuid.parse(decoder.decodeString())
}

@OptIn(ExperimentalUuidApi::class)
@Serializable
data class Listing(
    @Serializable(with = UuidSerializer::class)
    var id: Uuid?,
    var title: String,
    var description: String,
    var tradeable: Boolean,
    var insertionDate: LocalDateTime,
    @Serializable(with = UuidSerializer::class)
    var author: Uuid,
    var listingType: ListingType,
    @Serializable(with = UuidSerializer::class)
    var thumbnail: Uuid?,
    var identified_plant: Long?
)

enum class ListingType {
    Selling,
    Buying,
}

@Serializable
@OptIn(ExperimentalUuidApi::class)
data class User (
    @Serializable(with = UuidSerializer::class)
    var id: Uuid?,
    var location: Point?
)

@Serializable
data class Point(
    var x: Double,
    var y: Double,
)

@Serializable
data class Plant(
    var id: Int,
    var humanName: String,
    var species: String,
    var location: PlantLocation?,
    var produces_fruit: Boolean?,
    var description: String,
)

enum class PlantLocation {
    Outdoor,
    Indoor,
}
