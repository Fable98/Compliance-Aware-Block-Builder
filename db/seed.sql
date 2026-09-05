INSERT INTO sanctions_entities (name, source, list_type)
VALUES ('OFAC SDN - Digital Currency Addresses', 'OFAC_SDN', 'CRYPTO_ADDRESS')
RETURNING entity_id;
