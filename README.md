# LiRAYS-Scada

## General Schema
![General Schema](general_schema.png)

### Command examples
List Root
```json
{
	"LIST": {
		"cmd_id": "some-unique-id"
	}
}
```
List Folder
```json
{
	"LIST": {
		"cmd_id": "some-unique-id",
		"item_id": "6caf355f-0d80-44dc-b58c-4598d6c32244"
	}
}
```
Add Folders and Variables
```json
{
	"ADD": {
		"cmd_id": "some-unique-id",
		"parent_id": "6caf355f-0d80-44dc-b58c-4598d6c32244",
		"items_meta": [
			["fold1", "Folder", null],
			["fold2", "Folder", null],
			["var1", "Variable", "Float"],
			["var2", "Variable", "Integer"],
			["var3", "Variable", "Text"],
			["var4", "Variable", "Boolean"]
		]
	}
}
```
Set Variable Values
```json
{
	"SET": {
		"cmd_id": "some-unique-id",
		"var_ids_values": [
			["19d22f7f-6125-423c-8198-c7c5d41b8dea", {"Integer": 23}],
			["19d22f7f-6125-423c-8198-c7c5d41b8dea", {"Float": -90.12}],
			["19d22f7f-6125-423c-8198-c7c5d41b8dea", {"Text": "Some text"}],
			["19d22f7f-6125-423c-8198-c7c5d41b8dea", {"Boolean": true}]
		]
	}
}
```
Get Variable Values
```json
{
	"GET": {
		"cmd_id": "some-unique-id",
		"var_ids": [
			"19d22f7f-6125-423c-8198-c7c5d41b8dea",
			"6ead3b82-e643-434d-ba9c-568187e8b895"
		]
	}
}
```
Delete Folders and Variables
```json
{
	"DEL": {
		"cmd_id": "some-unique-id",
		"item_ids": [
			"19d22f7f-6125-423c-8198-c7c5d41b8dea",
			"6ead3b82-e643-434d-ba9c-568187e8b895"
		]
	}
}
```
