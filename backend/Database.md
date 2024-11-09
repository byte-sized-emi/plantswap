# Database schema

## What do we want to store?

 - users (stored through some OAuth2 identity provider, probably keycloak, maybe ory hydra?)
 - listings
 - uploaded images (maybe with identifications)
 - types of plants with plant information

## Offers

 - Person that offers something/is looking for something
 - Insertion date
 - Title
 - Description
 - type (Looking to buy/sell/give away/trade)
 - pictures
 - Plant information

## Plant information

 - normal name
 - species
 - location (indoor/outdoor)
 - watering information
 - produces fruit
 - plant season
 - tips/description
 - similar plants?

## Where do we want to store this?

It's all highly structured data which is easily represented as rows of data, making a database the best choice. Only exception to this is the similar plants field, which creates a graph out of plants. Because we only want to find the neighbours of each node (plant), postgres is still good enough for this.
