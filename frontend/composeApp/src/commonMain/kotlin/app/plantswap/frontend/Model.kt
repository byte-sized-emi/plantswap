@file:UseSerializers(UuidSerializer::class)
@file:OptIn(ExperimentalUuidApi::class)

package app.plantswap.frontend

import kotlinx.datetime.Clock
import kotlinx.datetime.Instant
import kotlinx.datetime.LocalDateTime
import kotlinx.serialization.KSerializer
import kotlinx.serialization.Serializable
import kotlinx.serialization.UseSerializers
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

private data object UuidSerializer : KSerializer<Uuid> {
    override val descriptor: SerialDescriptor = String.serializer().descriptor

    override fun serialize(encoder: Encoder, value: Uuid) =
        encoder.encodeString(value.toString())

    override fun deserialize(decoder: Decoder): Uuid =
        Uuid.parse(decoder.decodeString())
}

@Serializable
data class Listing(
    var id: Uuid? = null,
    var title: String,
    var description: String,
    var tradeable: Boolean,
    var insertionDate: Instant = Clock.System.now(),
    var author: Uuid,
    var listingType: ListingType,
    var pictures: List<Uuid> = emptyList(),
    var thumbnail: Uuid? = null,
    var identifiedPlant: Long? = null
)

enum class ListingType {
    Selling,
    Buying,
}

@Serializable
data class User (
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
