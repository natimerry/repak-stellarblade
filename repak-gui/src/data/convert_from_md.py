import json
def clean_text(text):
    return text.replace('\u2022', '').strip()

def parse_markdown_to_json(md_text):
    lines = md_text.strip().splitlines()
    results = []

    current_character = None
    current_skin_name = None

    for line in lines:
        line = clean_text(line)
        if line.startswith('## '):
            current_character = clean_text(line[3:])
        elif line.startswith('### '):
            current_skin_name = clean_text(line[4:])
        elif line.startswith('> '):
            skin_id = clean_text(line[2:])
            if current_character and current_skin_name and skin_id:
                results.append({
                    "name": current_character,
                    "skinid": skin_id,
                    "skin_name": current_skin_name
                })

    return results

# Example usage
if __name__ == "__main__":
    with open("character_data.md",'r') as f:
        markdown_text = f.read()

    parsed_json = parse_markdown_to_json(markdown_text)
    print(json.dumps(parsed_json, indent=4))
